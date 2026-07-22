# Isolated headless screenshots — never opens host VS Code.
# Uses code-server (VS Code in the browser) + Playwright.
#
#   ./scripts/screenshots/run-docker.sh

# --- Linux pydead binary ---
FROM rust:1-bookworm AS rust
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build -p pydead --release

# --- code-server + playwright ---
FROM mcr.microsoft.com/playwright:v1.51.0-jammy

USER root
ENV DEBIAN_FRONTEND=noninteractive

# code-server
RUN curl -fsSL https://code-server.dev/install.sh | sh

# xvfb + xterm + xdotool + imagemagick: true CLI window screenshots
RUN apt-get update && apt-get install -y --no-install-recommends \
    fonts-dejavu-core \
    xvfb xterm xdotool x11-utils imagemagick \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust /src/target/release/pydead /usr/local/bin/pydead
RUN chmod +x /usr/local/bin/pydead

WORKDIR /work
COPY vscode-extension /work/vscode-extension
COPY fixtures /work/fixtures
COPY scripts/screenshots /work/scripts/screenshots

# Compile extension + package VSIX (no vsce needed — code-server can load folder)
WORKDIR /work/vscode-extension
RUN npm install --no-fund --no-audit && npm run compile \
    && mkdir -p bin/linux-x64 \
    && cp /usr/local/bin/pydead bin/linux-x64/pydead \
    && chmod +x bin/linux-x64/pydead

# Install extension into code-server (folder name = publisher.name-version)
ENV XDG_DATA_HOME=/work/.local/share
RUN mkdir -p /work/.local/share/code-server/extensions \
    && EXT_VER="$(node -p "require('./package.json').version")" \
    && ln -s /work/vscode-extension \
       "/work/.local/share/code-server/extensions/pydead.pydead-${EXT_VER}"

WORKDIR /work/scripts/screenshots
RUN npm install --no-fund --no-audit \
    && npx playwright install chromium

ENV PYDEAD_BIN=/usr/local/bin/pydead
ENV PASSWORD=pydeadshot
ENV WORK_ROOT=/work
ENV OUT_DIR=/out
ENV EXT_PATH=/work/vscode-extension
ENV FIXTURE=/work/fixtures/sqlalchemy_project

EXPOSE 8080
ENTRYPOINT ["node", "capture-codeserver.mjs"]
