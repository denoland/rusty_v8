set DEPOT_TOOLS_WIN_TOOLCHAIN=0
set PATH=%~dp0\goog\depot_tools;%PATH%
set RUSTFLAGS=-C target-feature=+crt-static -C linker=%~dp0goog\v8\third_party\llvm-build\Release+Asserts\bin\lld-link.exe
