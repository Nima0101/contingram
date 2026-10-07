package io.contingram.platform;

import java.security.interfaces.RSAPublicKey;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.core.io.Resource;
import org.springframework.security.config.annotation.web.builders.HttpSecurity;
import org.springframework.security.config.http.SessionCreationPolicy;
import org.springframework.security.converter.RsaKeyConverters;
import org.springframework.security.oauth2.core.*;
import org.springframework.security.oauth2.jwt.*;
import org.springframework.security.web.SecurityFilterChain;

@Configuration
public class Security {
  @Bean
  JwtDecoder decoder(
      @Value("${platform.public-key}") Resource pem,
      @Value("${platform.issuer}") String issuer,
      @Value("${platform.audience}") String audience)
      throws java.io.IOException {
    RSAPublicKey key;
    try (var input = pem.getInputStream()) {
      key = RsaKeyConverters.x509().convert(input);
    }
    var decoder = NimbusJwtDecoder.withPublicKey(key).build();
    OAuth2TokenValidator<Jwt> claims =
        jwt ->
            jwt.getExpiresAt() != null
                    && jwt.getAudience().contains(audience)
                    && jwt.getSubject() != null
                    && jwt.getSubject().length() <= 200
                    && !jwt.getSubject().isBlank()
                ? OAuth2TokenValidatorResult.success()
                : OAuth2TokenValidatorResult.failure(new OAuth2Error("invalid_token"));
    decoder.setJwtValidator(
        new DelegatingOAuth2TokenValidator<>(
            JwtValidators.createDefaultWithIssuer(issuer), claims));
    return decoder;
  }

  @Bean
  SecurityFilterChain chain(HttpSecurity http) throws Exception {
    return http.sessionManagement(s -> s.sessionCreationPolicy(SessionCreationPolicy.STATELESS))
        .authorizeHttpRequests(
            a ->
                a.requestMatchers("/v1/admin/**")
                    .hasAuthority("SCOPE_admin")
                    .requestMatchers("/v1/**")
                    .hasAuthority("SCOPE_invoke")
                    .anyRequest()
                    .denyAll())
        .oauth2ResourceServer(o -> o.jwt(j -> {}))
        .build();
  }
}
