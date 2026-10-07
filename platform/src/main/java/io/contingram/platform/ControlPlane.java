package io.contingram.platform;

import com.fasterxml.jackson.databind.ObjectMapper;
import io.opentelemetry.api.OpenTelemetry;
import java.util.*;
import org.springframework.http.HttpStatus;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;
import org.springframework.web.server.ResponseStatusException;

@Service
public class ControlPlane {
  public record Tool(String id, String contract, String report, Integer risk) {}

  public record Intent(String key, String tool) {}

  public record Event(int version, UUID id, UUID intent, int seq, String kind, String detail) {}

  private final JdbcTemplate db;
  private final Verifier verifier;
  private final ObjectMapper json;
  private final OpenTelemetry telemetry;

  public ControlPlane(
      JdbcTemplate db, Verifier verifier, ObjectMapper json, OpenTelemetry telemetry) {
    this.db = db;
    this.verifier = verifier;
    this.json = json;
    this.telemetry = telemetry;
  }

  static void require(boolean condition, HttpStatus status) {
    if (!condition) throw new ResponseStatusException(status);
  }

  @Transactional
  public void register(Tool tool) {
    require(
        tool.id() != null
            && tool.id().matches("[a-zA-Z0-9_-]{1,64}")
            && tool.contract() != null
            && tool.report() != null
            && tool.risk() != null
            && tool.risk() >= 0
            && tool.risk() <= 100,
        HttpStatus.BAD_REQUEST);
    verifier.follow(tool.contract(), tool.report(), List.of());
    require(
        db.update(
                "INSERT INTO tools VALUES (?,?,?,?) ON CONFLICT DO NOTHING",
                tool.id(),
                tool.contract(),
                tool.report(),
                tool.risk())
            == 1,
        HttpStatus.CONFLICT);
  }

  @Transactional
  public Map<String, Object> intake(String subject, Intent request) {
    require(
        request.key() != null
            && request.key().matches("[a-zA-Z0-9_-]{1,128}")
            && request.tool() != null,
        HttpStatus.BAD_REQUEST);
    var tools = db.queryForList("SELECT risk FROM tools WHERE id=?", request.tool());
    require(!tools.isEmpty(), HttpStatus.NOT_FOUND);
    var id = UUID.randomUUID();
    String decision =
        ((Number) tools.getFirst().get("risk")).intValue() <= 30 ? "accepted" : "denied";
    int inserted =
        db.update(
            "INSERT INTO intents VALUES (?,?,?,?,?) ON CONFLICT(subject,idem) DO NOTHING",
            id,
            subject,
            request.key(),
            request.tool(),
            decision);
    var row =
        db.queryForMap(
            "SELECT id,tool,decision FROM intents WHERE subject=? AND idem=?",
            subject,
            request.key());
    require(row.get("tool").equals(request.tool()), HttpStatus.CONFLICT);
    if (inserted == 1) append(id, 1, "decision", decision);
    return row;
  }

  private void append(UUID intent, int seq, String kind, String detail) {
    var event = new Event(1, UUID.randomUUID(), intent, seq, kind, detail);
    try {
      db.update(
          "INSERT INTO audit(intent,seq,kind,detail) VALUES (?,?,?,?)", intent, seq, kind, detail);
      db.update(
          "INSERT INTO outbox(id,intent,seq,body) VALUES (?,?,?,?)",
          event.id(),
          intent,
          seq,
          json.writeValueAsString(event));
    } catch (com.fasterxml.jackson.core.JsonProcessingException e) {
      throw new IllegalStateException(e);
    }
  }

  @Transactional
  public String recover(String subject, UUID id, List<String> observations) {
    var rows =
        db.queryForList(
            "SELECT i.decision,t.contract,t.report FROM intents i JOIN tools t ON t.id=i.tool WHERE i.id=? AND i.subject=? FOR UPDATE OF i",
            id,
            subject);
    require(!rows.isEmpty(), HttpStatus.NOT_FOUND);
    var row = rows.getFirst();
    require(row.get("decision").equals("accepted"), HttpStatus.FORBIDDEN);
    String result =
        Telemetry.measure(
            telemetry,
            () ->
                verifier.follow(
                    (String) row.get("contract"), (String) row.get("report"), observations));
    int seq =
        db.queryForObject(
            "SELECT COALESCE(MAX(seq),0)+1 FROM audit WHERE intent=?", Integer.class, id);
    append(id, seq, "recovery", result);
    return result;
  }

  public List<Map<String, Object>> audit(String subject, UUID id) {
    require(
        Boolean.TRUE.equals(
            db.queryForObject(
                "SELECT EXISTS(SELECT 1 FROM intents WHERE id=? AND subject=?)",
                Boolean.class,
                id,
                subject)),
        HttpStatus.NOT_FOUND);
    return db.queryForList("SELECT seq,kind,detail FROM audit WHERE intent=? ORDER BY seq", id);
  }
}
