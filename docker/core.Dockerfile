# syntax=docker/dockerfile:1

FROM rust:1.98-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY supabase/migrations supabase/migrations
COPY services/core services/core
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    mkdir -p /out \
    && cargo build --release -p docufill-core --bin api --bin worker \
    && cp target/release/api /out/api \
    && cp target/release/worker /out/worker

FROM debian:bookworm-slim
ARG TARGETARCH

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        fonts-noto-core \
        libatomic1 \
        libstdc++6 \
        tar \
    && rm -rf /var/lib/apt/lists/* \
    && test -f /usr/share/fonts/truetype/noto/NotoSans-Regular.ttf \
    && case "${TARGETARCH}" in \
         amd64) pdfium_arch=x64 ;; \
         arm64) pdfium_arch=arm64 ;; \
         *) echo "unsupported TARGETARCH=${TARGETARCH}" && exit 1 ;; \
       esac \
    && curl -fsSL -o /tmp/pdfium.tgz \
         "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F8035/pdfium-linux-${pdfium_arch}.tgz" \
    && mkdir -p /opt/pdfium \
    && tar -xzf /tmp/pdfium.tgz -C /opt/pdfium \
    && rm /tmp/pdfium.tgz \
    && test -f /opt/pdfium/lib/libpdfium.so \
    && useradd --system --uid 10001 --home /nonexistent --shell /usr/sbin/nologin docufill

COPY --from=builder /out/api /usr/local/bin/api
COPY --from=builder /out/worker /usr/local/bin/worker

ENV PDFIUM_LIB_PATH=/opt/pdfium/lib \
    UNICODE_FONT_PATH=/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf \
    LD_LIBRARY_PATH=/opt/pdfium/lib \
    API_BIND=0.0.0.0:8080

USER docufill
EXPOSE 8080
CMD ["api"]
