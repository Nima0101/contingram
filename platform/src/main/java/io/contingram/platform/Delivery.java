package io.contingram.platform;

import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.*;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.kafka.annotation.KafkaListener;
import org.springframework.kafka.core.KafkaTemplate;
import org.springframework.scheduling.annotation.Scheduled;
import org.springframework.stereotype.Service;
import org.springframework.transaction.support.TransactionTemplate;

@Service
public class Delivery {
  private final JdbcTemplate db;
  private final ObjectMapper json;
  private final KafkaTemplate<String, String> kafka;
  private final TransactionTemplate tx;

  public Delivery(
      JdbcTemplate db,
      ObjectMapper json,
      KafkaTemplate<String, String> kafka,
      org.springframework.transaction.PlatformTransactionManager manager) {
    this.db = db;
    this.json = json;
    this.kafka = kafka;
    this.tx = new TransactionTemplate(manager);
  }

  @org.springframework.context.annotation.Bean
  org.springframework.kafka.listener.CommonErrorHandler errors() {
    return new org.springframework.kafka.listener.DefaultErrorHandler(
        new org.springframework.util.backoff.FixedBackOff(
            1000, org.springframework.util.backoff.FixedBackOff.UNLIMITED_ATTEMPTS));
  }

  @Scheduled(fixedDelayString = "${platform.publish-delay:1000}")
  public void publish() {
    tx.executeWithoutResult(
        status -> {
          for (var row :
              db.queryForList(
                  "SELECT id,intent,body FROM outbox WHERE NOT sent ORDER BY intent,seq LIMIT 100 FOR UPDATE SKIP LOCKED")) {
            try {
              kafka
                  .send(
                      "contingram.events.v1",
                      row.get("intent").toString(),
                      (String) row.get("body"))
                  .get(6, java.util.concurrent.TimeUnit.SECONDS);
            } catch (InterruptedException e) {
              Thread.currentThread().interrupt();
              throw new IllegalStateException("publication interrupted", e);
            } catch (java.util.concurrent.ExecutionException
                | java.util.concurrent.TimeoutException e) {
              throw new IllegalStateException("publication unconfirmed; retry retained", e);
            }
            db.update("UPDATE outbox SET sent=true WHERE id=?", row.get("id"));
          }
        });
  }

  @KafkaListener(topics = "contingram.events.v1", autoStartup = "${platform.consumer:true}")
  public void receive(String body) {
    // Authenticated broker writers are trusted for transport; payloads must exactly match the local
    // outbox.
    tx.executeWithoutResult(
        status -> {
          try {
            var event = json.readValue(body, ControlPlane.Event.class);
            if (event.version() != 1
                || event.id() == null
                || event.intent() == null
                || event.seq() < 1) throw new IllegalArgumentException("unsupported event");
            if (!Boolean.TRUE.equals(
                db.queryForObject(
                    "SELECT EXISTS(SELECT 1 FROM outbox WHERE id=? AND body=?)",
                    Boolean.class,
                    event.id(),
                    body))) throw new IllegalArgumentException("event not in authoritative outbox");
            db.update(
                "INSERT INTO projections VALUES (?,0) ON CONFLICT DO NOTHING", event.intent());
            int seq =
                db.queryForObject(
                    "SELECT seq FROM projections WHERE intent=? FOR UPDATE",
                    Integer.class,
                    event.intent());
            if (Boolean.TRUE.equals(
                db.queryForObject(
                    "SELECT EXISTS(SELECT 1 FROM inbox WHERE id=?)", Boolean.class, event.id())))
              return;
            boolean next = event.seq() == seq + 1;
            db.update(
                "INSERT INTO inbox VALUES (?,?,?,?,?,?)",
                event.id(),
                event.intent(),
                event.seq(),
                body,
                next ? "applied" : "dead",
                next ? "" : "sequence_gap");
            if (next)
              db.update("UPDATE projections SET seq=? WHERE intent=?", event.seq(), event.intent());
          } catch (com.fasterxml.jackson.core.JsonProcessingException e) {
            throw new IllegalArgumentException("malformed event", e);
          }
        });
  }

  public void replay(UUID id) {
    tx.executeWithoutResult(
        status -> {
          var rows =
              db.queryForList("SELECT body,intent,seq FROM inbox WHERE id=? AND state='dead'", id);
          ControlPlane.require(!rows.isEmpty(), org.springframework.http.HttpStatus.NOT_FOUND);
          var row = rows.getFirst();
          int seq =
              db.queryForObject(
                  "SELECT seq FROM projections WHERE intent=? FOR UPDATE",
                  Integer.class,
                  row.get("intent"));
          ControlPlane.require(
              ((Number) row.get("seq")).intValue() == seq + 1,
              org.springframework.http.HttpStatus.CONFLICT);
          db.update("UPDATE inbox SET state='applied',reason='' WHERE id=?", id);
          db.update("UPDATE projections SET seq=? WHERE intent=?", seq + 1, row.get("intent"));
        });
  }
}
