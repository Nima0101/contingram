package io.contingram.platform;

import java.util.*;
import org.springframework.security.core.annotation.AuthenticationPrincipal;
import org.springframework.security.oauth2.jwt.Jwt;
import org.springframework.web.bind.annotation.*;

@RestController
@RequestMapping("/v1")
public class Api {
  private final ControlPlane service;
  private final Delivery delivery;

  public Api(ControlPlane service, Delivery delivery) {
    this.service = service;
    this.delivery = delivery;
  }

  @PostMapping("/admin/tools")
  public void register(@RequestBody ControlPlane.Tool tool) {
    service.register(tool);
  }

  @PostMapping("/intents")
  public Map<String, Object> intake(
      @AuthenticationPrincipal Jwt jwt, @RequestBody ControlPlane.Intent intent) {
    return service.intake(jwt.getSubject(), intent);
  }

  @PostMapping("/intents/{id}/recovery")
  public String recover(
      @AuthenticationPrincipal Jwt jwt,
      @PathVariable UUID id,
      @RequestBody List<String> observations) {
    return service.recover(jwt.getSubject(), id, observations);
  }

  @GetMapping("/intents/{id}/audit")
  public List<Map<String, Object>> audit(@AuthenticationPrincipal Jwt jwt, @PathVariable UUID id) {
    return service.audit(jwt.getSubject(), id);
  }

  @PostMapping("/admin/replay/{id}")
  public void replay(@PathVariable UUID id) {
    delivery.replay(id);
  }
}
