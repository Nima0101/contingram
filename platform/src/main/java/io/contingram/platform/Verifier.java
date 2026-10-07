package io.contingram.platform;

import java.nio.file.*;
import java.util.*;
import java.util.concurrent.TimeUnit;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.http.HttpStatus;
import org.springframework.stereotype.Component;
import org.springframework.web.server.ResponseStatusException;

@Component
public class Verifier {
  private final String binary;

  public Verifier(@Value("${platform.binary}") String binary) {
    this.binary = binary;
  }

  public String follow(String contract, String report, List<String> observations) {
    if (contract.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > 65536
        || report.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > 65536
        || observations.size() > 32
        || observations.stream().anyMatch(o -> o == null || !o.matches("[a-zA-Z0-9_-]{1,64}")))
      throw new ResponseStatusException(HttpStatus.BAD_REQUEST, "artifact bounds");
    Path dir = null;
    Process process = null;
    try {
      dir = Files.createTempDirectory("contingram-");
      var model = Files.writeString(dir.resolve("contract.json"), contract);
      var policy = Files.writeString(dir.resolve("report.json"), report);
      var output = dir.resolve("out");
      var args = new ArrayList<>(List.of(binary, "follow", model.toString(), policy.toString()));
      args.addAll(observations);
      process =
          new ProcessBuilder(args)
              .redirectOutput(output.toFile())
              .redirectError(ProcessBuilder.Redirect.DISCARD)
              .start();
      if (!process.waitFor(5, TimeUnit.SECONDS)) {
        process.destroyForcibly().waitFor();
        throw new ResponseStatusException(HttpStatus.UNPROCESSABLE_ENTITY, "verifier timeout");
      }
      if (process.exitValue() != 0 || Files.size(output) > 4096)
        throw new ResponseStatusException(HttpStatus.UNPROCESSABLE_ENTITY, "unverified recovery");
      return Files.readString(output).strip();
    } catch (java.io.IOException e) {
      throw new ResponseStatusException(HttpStatus.SERVICE_UNAVAILABLE, "verifier unavailable");
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new ResponseStatusException(HttpStatus.SERVICE_UNAVAILABLE);
    } finally {
      if (process != null && process.isAlive()) process.destroyForcibly();
      if (dir != null) {
        try (var paths = Files.list(dir)) {
          for (var path : paths.toList()) Files.delete(path);
          Files.delete(dir);
        } catch (java.io.IOException e) {
          throw new IllegalStateException("temporary artifact cleanup failed", e);
        }
      }
    }
  }
}
