MUSL_VER = 1.2.4
GCC_VER  = 11.2.0

GCC_CONFIG += --enable-default-pie

DL_CMD   = curl -C - -L -s -o
SHA1_CMD = shasum -a 1 -c
