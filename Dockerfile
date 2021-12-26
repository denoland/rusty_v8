FROM rustembedded/cross:aarch64-linux-android-0.2.1

RUN apt update && \
    apt install -y curl && \
    curl -L https://github.com/mozilla/sccache/releases/download/v0.2.15/sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz | tar xzf -

ENV TZ=Etc/UTC
COPY ./build/*.sh /chromium_build/
RUN \
	DEBIAN_FRONTEND=noninteractive \
	ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone \
	&& apt-get update && apt-get install -y lsb-release sudo \
	&& /chromium_build/install-build-deps-android.sh \
	&& rm -rf /chromium_build \
	&& rm -rf /var/lib/apt/lists/*

RUN chmod +x /sccache-v0.2.15-x86_64-unknown-linux-musl/sccache

ENV SCCACHE=/sccache-v0.2.15-x86_64-unknown-linux-musl/sccache
ENV SCCACHE_DIR=./target/sccache
