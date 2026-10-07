package io.contingram.platform;

import io.opentelemetry.api.OpenTelemetry;
import io.opentelemetry.exporter.logging.LoggingMetricExporter;
import io.opentelemetry.exporter.logging.LoggingSpanExporter;
import io.opentelemetry.sdk.OpenTelemetrySdk;
import io.opentelemetry.sdk.metrics.SdkMeterProvider;
import io.opentelemetry.sdk.metrics.export.PeriodicMetricReader;
import io.opentelemetry.sdk.trace.SdkTracerProvider;
import io.opentelemetry.sdk.trace.export.SimpleSpanProcessor;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

@Configuration
public class Telemetry {
  @Bean(destroyMethod = "close")
  OpenTelemetrySdk sdk() {
    return OpenTelemetrySdk.builder()
        .setTracerProvider(
            SdkTracerProvider.builder()
                .addSpanProcessor(SimpleSpanProcessor.create(LoggingSpanExporter.create()))
                .build())
        .setMeterProvider(
            SdkMeterProvider.builder()
                .registerMetricReader(
                    PeriodicMetricReader.builder(LoggingMetricExporter.create()).build())
                .build())
        .build();
  }

  static String measure(OpenTelemetry telemetry, java.util.function.Supplier<String> action) {
    var span =
        telemetry.getTracer("contingram.platform").spanBuilder("recovery.decision").startSpan();
    try {
      var result = action.get();
      telemetry.getMeter("contingram.platform").counterBuilder("recovery.decisions").build().add(1);
      return result;
    } catch (RuntimeException ex) {
      span.setStatus(io.opentelemetry.api.trace.StatusCode.ERROR);
      throw ex;
    } finally {
      span.end();
    }
  }
}
