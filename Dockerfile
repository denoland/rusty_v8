# sccache
FROM ubuntu:16.04 AS sccache

ENV TZ=Etc/UTC
RUN \
	DEBIAN_FRONTEND=noninteractive \
	ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone \
	&& apt-get update && apt-get install -y curl \
	&& rm -rf /var/lib/apt/lists/*

ARG sccache_version="0.2.12"
ARG sccache_platform="x86_64-unknown-linux-musl"
ARG sccache_basename="sccache-$sccache_version-$sccache_platform"
ARG sccache_url="https://github.com/mozilla/sccache/releases/download/$sccache_version/$sccache_basename.tar.gz"
RUN \
	cd / \
	&& echo $sccache_url \
	&& curl -LO "$sccache_url" \
	&& tar -xzvf "$sccache_basename.tar.gz" \
	&& mv $sccache_basename/sccache /usr/local/bin/sccache \
	&& rm -rf $sccache_basename

# aarch64-linux-android
FROM rustembedded/cross:aarch64-linux-android AS aarch64-linux-android

ENV TZ=Etc/UTC
COPY ./build/*.sh /chromium_build/
RUN \
	DEBIAN_FRONTEND=noninteractive \
	ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone \
	&& apt-get update && apt-get install -y lsb-release sudo \
	&& /chromium_build/install-build-deps-android.sh \
	&& rm -rf /chromium_build \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=sccache /usr/local/bin/sccache /usr/local/bin/sccache
ENV SCCACHE_DIR=./target/sccache

# x86_64-linux-android
FROM rustembedded/cross:x86_64-linux-android AS x86_64-linux-android

ENV TZ=Etc/UTC
COPY ./build/*.sh /chromium_build/
RUN \
	DEBIAN_FRONTEND=noninteractive \
	ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone \
	&& apt-get update && apt-get install -y lsb-release sudo \
	&& /chromium_build/install-build-deps-android.sh \
	&& rm -rf /chromium_build \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=sccache /usr/local/bin/sccache /usr/local/bin/sccache
ENV SCCACHE_DIR=./target/sccache
