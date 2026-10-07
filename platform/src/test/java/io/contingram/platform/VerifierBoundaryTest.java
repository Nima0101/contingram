package io.contingram.platform;

import static org.assertj.core.api.Assertions.*;

import java.util.List;
import org.junit.jupiter.api.Test;
import org.springframework.http.HttpStatus;
import org.springframework.web.server.ResponseStatusException;

class VerifierBoundaryTest {
  @Test
  void rejectsUnsafeArgumentsBeforeStartingAnyProcess() {
    var verifier = new Verifier("/nonexistent-verifier");
    for (String observation :
        List.of("--help", "-x", "a b", "a;id", "$(id)", "a\nb", "", "a".repeat(65))) {
      assertThatThrownBy(() -> verifier.follow("{}", "{}", List.of(observation)))
          .isInstanceOfSatisfying(
              ResponseStatusException.class,
              e -> assertThat(e.getStatusCode()).isEqualTo(HttpStatus.BAD_REQUEST));
    }
    assertThatThrownBy(() -> verifier.follow("{}", "{}", List.of("receipt_ok-1")))
        .isInstanceOfSatisfying(
            ResponseStatusException.class,
            e -> assertThat(e.getStatusCode()).isEqualTo(HttpStatus.SERVICE_UNAVAILABLE));
  }
}
