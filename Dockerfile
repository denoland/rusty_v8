ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

RUN apt update && \
    apt install -y curl && \
    curl -L https://github.com/mozilla/sccache/releases/download/v0.7.7/sccache-v0.7.7-x86_64-unknown-linux-musl.tar.gz | tar xzf -

ENV TZ=Etc/UTC
COPY ./build/*.sh /chromium_build/
RUN \
	DEBIAN_FRONTEND=noninteractive \
	ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone \
	&& apt-get update && apt-get install -y lsb-release sudo \
	&& sed -i 's/snapcraft/snapcraftnoinstall/g' /chromium_build/install-build-deps.sh \
	&& /chromium_build/install-build-deps.sh --no-prompt --no-chromeos-fonts \
	&& rm -rf /chromium_build \
	&& rm -rf /var/lib/apt/lists/*

RUN chmod +x /sccache-v0.7.7-x86_64-unknown-linux-musl/sccache

ENV SCCACHE=/sccache-v0.7.7-x86_64-unknown-linux-musl/sccache
ENV SCCACHE_DIR=./target/sccache
