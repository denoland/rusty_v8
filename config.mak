MUSL_VER = 1.1.24
GCC_VER  = 9.2.0

GCC_CONFIG += --enable-default-pie

DL_CMD   = curl -C - -L -s -o
SHA1_CMD = shasum -a 1 -c
