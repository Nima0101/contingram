FROM rust:1.90-bookworm AS verifier
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples
COPY docs ./docs
COPY LICENSE NOTICE ./
RUN cargo build --locked --release
FROM eclipse-temurin:21-jre-noble
WORKDIR /app
COPY --from=verifier /src/target/release/contingram /app/contingram
COPY platform/target/control-plane-0.1.0-SNAPSHOT.jar /app/service.jar
ENV CONTINGRAM_BINARY=/app/contingram
USER 10001:10001
ENTRYPOINT ["java", "-jar", "/app/service.jar"]
