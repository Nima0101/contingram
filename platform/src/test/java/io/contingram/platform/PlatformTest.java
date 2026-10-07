package io.contingram.platform;

import static org.assertj.core.api.Assertions.*;
import static org.springframework.test.web.servlet.request.MockMvcRequestBuilders.*;
import static org.springframework.test.web.servlet.result.MockMvcResultMatchers.*;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.nimbusds.jose.*;
import com.nimbusds.jose.crypto.RSASSASigner;
import com.nimbusds.jwt.*;
import java.nio.file.*;
import java.security.*;
import java.time.Instant;
import java.util.*;
import org.apache.kafka.clients.consumer.*;
import org.apache.kafka.common.serialization.StringDeserializer;
import org.junit.jupiter.api.*;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.autoconfigure.web.servlet.AutoConfigureMockMvc;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.test.context.DynamicPropertyRegistry;
import org.springframework.test.context.DynamicPropertySource;
import org.springframework.test.web.servlet.MockMvc;
import org.testcontainers.containers.PostgreSQLContainer;
import org.testcontainers.kafka.KafkaContainer;

@SpringBootTest
@AutoConfigureMockMvc
class PlatformTest {
  static final PostgreSQLContainer<?> POSTGRES = new PostgreSQLContainer<>("postgres:17.6-alpine");
  static final KafkaContainer KAFKA = new KafkaContainer("apache/kafka-native:3.9.1");
  static final KeyPair KEYS;
  static final Path PEM;

  static {
    try {
      var generator = KeyPairGenerator.getInstance("RSA");
      generator.initialize(2048);
      KEYS = generator.generateKeyPair();
      PEM = Files.createTempFile("issuer-", ".pem");
      Files.writeString(
          PEM,
          "-----BEGIN PUBLIC KEY-----\n"
              + Base64.getMimeEncoder(64, new byte[] {10})
                  .encodeToString(KEYS.getPublic().getEncoded())
              + "\n-----END PUBLIC KEY-----\n");
      POSTGRES.start();
      KAFKA.start();
    } catch (Exception e) {
      throw new ExceptionInInitializerError(e);
    }
  }

  @DynamicPropertySource
  static void properties(DynamicPropertyRegistry p) {
    p.add("spring.datasource.url", POSTGRES::getJdbcUrl);
    p.add("spring.datasource.username", POSTGRES::getUsername);
    p.add("spring.datasource.password", POSTGRES::getPassword);
    p.add("spring.kafka.bootstrap-servers", KAFKA::getBootstrapServers);
    p.add("platform.public-key", () -> PEM.toUri().toString());
    p.add("platform.issuer", () -> "https://issuer.example");
    p.add("platform.binary", () -> System.getenv("CONTINGRAM_BINARY"));
    p.add("platform.publish-delay", () -> "3600000");
    p.add("platform.consumer", () -> "false");
  }

  @Autowired MockMvc http;
  @Autowired ControlPlane plane;
  @Autowired Delivery delivery;
  @Autowired JdbcTemplate db;
  @Autowired ObjectMapper json;
  @Autowired Verifier verifier;

  static String token(String subject, String scope, String audience, String issuer, Instant expiry)
      throws Exception {
    var jwt =
        new SignedJWT(
            new JWSHeader(JWSAlgorithm.RS256),
            new JWTClaimsSet.Builder()
                .subject(subject)
                .issuer(issuer)
                .audience(audience)
                .expirationTime(expiry == null ? null : Date.from(expiry))
                .claim("scope", scope)
                .build());
    jwt.sign(new RSASSASigner(KEYS.getPrivate()));
    return "Bearer " + jwt.serialize();
  }

  static String token(String subject, String scope) throws Exception {
    return token(
        subject,
        scope,
        "contingram-platform",
        "https://issuer.example",
        Instant.now().plusSeconds(300));
  }

  static String forgedToken() throws Exception {
    var parts = token("alice", "invoke").split("\\.");
    parts[2] = (parts[2].startsWith("A") ? "B" : "A") + parts[2].substring(1);
    return String.join(".", parts);
  }

  ControlPlane.Tool tool(String name, int risk) throws Exception {
    String contract = Files.readString(Path.of("../examples/contracts/receipt.json"));
    var output = Files.createTempFile("policy-", ".json");
    try {
      var process =
          new ProcessBuilder(
                  System.getenv("CONTINGRAM_BINARY"),
                  "analyze",
                  "../examples/contracts/receipt.json",
                  "--horizon",
                  "2")
              .redirectOutput(output.toFile())
              .start();
      assertThat(process.waitFor()).isZero();
      return new ControlPlane.Tool(name, contract, Files.readString(output), risk);
    } finally {
      Files.delete(output);
    }
  }

  @AfterAll
  static void cleanup() throws Exception {
    POSTGRES.stop();
    KAFKA.stop();
    Files.delete(PEM);
  }

  @Test
  void ambientCookiesCannotBypassCsrf() throws Exception {
    http.perform(get("/v1/intents/missing/audit")).andExpect(status().isUnauthorized());
    http.perform(
            post("/v1/intents")
                .cookie(
                    new jakarta.servlet.http.Cookie(
                        "access_token", token("alice", "invoke").substring(7)))
                .contentType("application/json")
                .content("{\"key\":\"cookie-only\",\"tool\":\"missing\"}"))
        .andExpect(status().isForbidden());
    assertThat(
            db.queryForObject(
                "SELECT count(*) FROM intents WHERE idem='cookie-only'", Integer.class))
        .isZero();
  }

  @Test
  void authorizationAndDurableIntake() throws Exception {
    var tool = tool("receipt", 10);
    http.perform(
            post("/v1/admin/tools")
                .contentType("application/json")
                .content(json.writeValueAsString(tool)))
        .andExpect(status().isForbidden());
    http.perform(
            post("/v1/admin/tools")
                .header("Authorization", token("alice", "invoke"))
                .contentType("application/json")
                .content(json.writeValueAsString(tool)))
        .andExpect(status().isForbidden());
    assertThat(db.queryForObject("SELECT count(*) FROM tools WHERE id='receipt'", Integer.class))
        .isZero();
    http.perform(
            post("/v1/admin/tools")
                .header("Authorization", token("admin", "admin"))
                .contentType("application/json")
                .content(json.writeValueAsString(tool)))
        .andExpect(status().isOk());
    http.perform(
            post("/v1/admin/tools")
                .header("Authorization", token("admin", "admin"))
                .contentType("application/json")
                .content(json.writeValueAsString(tool)))
        .andExpect(status().isConflict());
    var missingRisk = json.valueToTree(tool);
    ((com.fasterxml.jackson.databind.node.ObjectNode) missingRisk).remove("risk");
    http.perform(
            post("/v1/admin/tools")
                .header("Authorization", token("admin", "admin"))
                .contentType("application/json")
                .content(json.writeValueAsString(missingRisk)))
        .andExpect(status().isBadRequest());
    for (String bad :
        List.of(
            token("alice", "invoke", "contingram-platform", "https://issuer.example", null),
            "Bearer invalid",
            forgedToken(),
            token(
                "alice",
                "invoke",
                "wrong",
                "https://issuer.example",
                Instant.now().plusSeconds(300)),
            token(
                "alice",
                "invoke",
                "contingram-platform",
                "https://wrong.example",
                Instant.now().plusSeconds(300)),
            token(
                "alice",
                "invoke",
                "contingram-platform",
                "https://issuer.example",
                Instant.now().minusSeconds(300)))) {
      http.perform(
              post("/v1/intents")
                  .header("Authorization", bad)
                  .contentType("application/json")
                  .content("{\"key\":\"unauthorized\",\"tool\":\"receipt\"}"))
          .andExpect(status().isUnauthorized());
    }
    assertThat(
            db.queryForObject(
                "SELECT count(*) FROM intents WHERE idem='unauthorized'", Integer.class))
        .isZero();
    var body = "{\"key\":\"same\",\"tool\":\"receipt\"}";
    var first =
        http.perform(
                post("/v1/intents")
                    .header("Authorization", token("alice", "invoke"))
                    .contentType("application/json")
                    .content(body))
            .andExpect(status().isOk())
            .andReturn()
            .getResponse()
            .getContentAsString();
    http.perform(
            post("/v1/intents")
                .header("Authorization", token("alice", "invoke"))
                .contentType("application/json")
                .content(body))
        .andExpect(content().json(first));
    UUID id = UUID.fromString(json.readTree(first).get("id").asText());
    assertThat(db.queryForObject("SELECT count(*) FROM audit WHERE intent=?", Integer.class, id))
        .isEqualTo(1);
    http.perform(
            get("/v1/intents/" + id + "/audit").header("Authorization", token("bob", "invoke")))
        .andExpect(status().isNotFound());
    http.perform(
            post("/v1/intents/" + id + "/recovery")
                .header("Authorization", token("alice", "invoke"))
                .contentType("application/json")
                .content("[]"))
        .andExpect(status().isOk())
        .andExpect(content().string(org.hamcrest.Matchers.containsString("status")));
    http.perform(
            post("/v1/intents/" + id + "/recovery")
                .header("Authorization", token("alice", "invoke"))
                .contentType("application/json")
                .content("[\"impossible\"]"))
        .andExpect(status().isUnprocessableEntity());
    http.perform(
            get("/v1/intents/" + id + "/audit").header("Authorization", token("alice", "invoke")))
        .andExpect(status().isOk())
        .andExpect(jsonPath("$.length()").value(2));
    plane.register(tool("high-risk", 90));
    assertThatThrownBy(() -> plane.intake("alice", new ControlPlane.Intent("same", "high-risk")))
        .isInstanceOf(org.springframework.web.server.ResponseStatusException.class);
    var denied = plane.intake("alice", new ControlPlane.Intent("denied", "high-risk"));
    assertThat(denied.get("decision")).isEqualTo("denied");
    assertThatThrownBy(() -> plane.recover("alice", (UUID) denied.get("id"), List.of()))
        .isInstanceOf(org.springframework.web.server.ResponseStatusException.class);
    assertThatThrownBy(() -> db.update("DELETE FROM audit WHERE intent=?", id))
        .isInstanceOf(org.springframework.dao.DataAccessException.class);
    assertThatThrownBy(() -> verifier.follow(tool.contract(), "{}", List.of()))
        .isInstanceOf(org.springframework.web.server.ResponseStatusException.class);
  }

  @Test
  void concurrentDuplicatesAndEventRecovery() throws Exception {
    plane.register(tool("events", 10));
    try (var workers = java.util.concurrent.Executors.newFixedThreadPool(8)) {
      var tasks = new ArrayList<java.util.concurrent.Callable<Map<String, Object>>>();
      for (int i = 0; i < 16; i++)
        tasks.add(() -> plane.intake("worker", new ControlPlane.Intent("duplicate", "events")));
      var ids = new HashSet<Object>();
      for (var result : workers.invokeAll(tasks)) ids.add(result.get().get("id"));
      assertThat(ids).hasSize(1);
    }
    UUID id =
        (UUID) plane.intake("worker", new ControlPlane.Intent("duplicate", "events")).get("id");
    assertThat(db.queryForObject("SELECT count(*) FROM outbox WHERE intent=?", Integer.class, id))
        .isEqualTo(1);
    plane.recover("worker", id, List.of("absent-final"));
    var events = db.queryForList("SELECT id,body FROM outbox WHERE intent=? ORDER BY seq", id);
    delivery.receive((String) events.get(1).get("body"));
    assertThat(
            db.queryForObject(
                "SELECT state FROM inbox WHERE id=?", String.class, events.get(1).get("id")))
        .isEqualTo("dead");
    assertThatThrownBy(() -> delivery.replay((UUID) events.get(1).get("id")))
        .isInstanceOf(org.springframework.web.server.ResponseStatusException.class);
    delivery.receive((String) events.getFirst().get("body"));
    delivery.receive((String) events.getFirst().get("body"));
    delivery.replay((UUID) events.get(1).get("id"));
    delivery.receive((String) events.get(1).get("body"));
    assertThat(db.queryForObject("SELECT seq FROM projections WHERE intent=?", Integer.class, id))
        .isEqualTo(2);
    assertThat(db.queryForObject("SELECT count(*) FROM inbox WHERE intent=?", Integer.class, id))
        .isEqualTo(2);
    assertThatThrownBy(
            () ->
                delivery.receive(
                    ((String) events.getFirst().get("body")).replace("decision", "forged")))
        .isInstanceOf(IllegalArgumentException.class);
    assertThatThrownBy(() -> delivery.receive("{}")).isInstanceOf(IllegalArgumentException.class);
    // Broker acknowledgment followed by lost DB acknowledgment: redeliver all rows.
    delivery.publish();
    db.update("UPDATE outbox SET sent=false WHERE intent=?", id);
    delivery.publish();
    var props = new Properties();
    props.put(ConsumerConfig.BOOTSTRAP_SERVERS_CONFIG, KAFKA.getBootstrapServers());
    props.put(ConsumerConfig.GROUP_ID_CONFIG, "test-" + UUID.randomUUID());
    props.put(ConsumerConfig.AUTO_OFFSET_RESET_CONFIG, "earliest");
    try (var consumer =
        new KafkaConsumer<String, String>(
            props, new StringDeserializer(), new StringDeserializer())) {
      consumer.subscribe(List.of("contingram.events.v1"));
      int count = 0;
      long until = System.nanoTime() + java.time.Duration.ofSeconds(20).toNanos();
      while (count < 4 && System.nanoTime() < until)
        for (var record : consumer.poll(java.time.Duration.ofMillis(300))) {
          delivery.receive(record.value());
          if (record.key().equals(id.toString())) count++;
        }
      assertThat(count).isEqualTo(4);
    }
    assertThat(db.queryForObject("SELECT count(*) FROM inbox WHERE intent=?", Integer.class, id))
        .isEqualTo(2);
  }

  @Test
  void brokerOutageRetainsOutbox() throws Exception {
    plane.register(tool("outage", 10));
    UUID id = (UUID) plane.intake("outage", new ControlPlane.Intent("one", "outage")).get("id");
    KAFKA.getDockerClient().pauseContainerCmd(KAFKA.getContainerId()).exec();
    try {
      assertThatThrownBy(delivery::publish).isInstanceOf(IllegalStateException.class);
      assertThat(db.queryForObject("SELECT sent FROM outbox WHERE intent=?", Boolean.class, id))
          .isFalse();
    } finally {
      KAFKA.getDockerClient().unpauseContainerCmd(KAFKA.getContainerId()).exec();
    }
    delivery.publish();
    assertThat(db.queryForObject("SELECT sent FROM outbox WHERE intent=?", Boolean.class, id))
        .isTrue();
  }

  @Test
  void telemetryHasRealSpansAndMetrics() {
    var spans = io.opentelemetry.sdk.testing.exporter.InMemorySpanExporter.create();
    var metrics = io.opentelemetry.sdk.testing.exporter.InMemoryMetricReader.create();
    try (var sdk =
        io.opentelemetry.sdk.OpenTelemetrySdk.builder()
            .setTracerProvider(
                io.opentelemetry.sdk.trace.SdkTracerProvider.builder()
                    .addSpanProcessor(
                        io.opentelemetry.sdk.trace.export.SimpleSpanProcessor.create(spans))
                    .build())
            .setMeterProvider(
                io.opentelemetry.sdk.metrics.SdkMeterProvider.builder()
                    .registerMetricReader(metrics)
                    .build())
            .build()) {
      assertThat(Telemetry.measure(sdk, () -> "decision")).isEqualTo("decision");
      assertThatThrownBy(
              () ->
                  Telemetry.measure(
                      sdk,
                      () -> {
                        throw new IllegalStateException("fault");
                      }))
          .isInstanceOf(IllegalStateException.class);
      assertThat(spans.getFinishedSpanItems()).hasSize(2);
      assertThat(spans.getFinishedSpanItems().get(1).getStatus().getStatusCode())
          .isEqualTo(io.opentelemetry.api.trace.StatusCode.ERROR);
      assertThat(metrics.collectAllMetrics())
          .anySatisfy(
              m -> {
                assertThat(m.getName()).isEqualTo("recovery.decisions");
                assertThat(m.getLongSumData().getPoints().iterator().next().getValue())
                    .isEqualTo(1);
              });
    }
  }
}
