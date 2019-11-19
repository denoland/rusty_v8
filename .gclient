solutions = [
    {
        'url': 'https://chromium.googlesource.com/v8/v8.git@7.9.317.12',
        'name': 'v8',
        'custom_hooks': [
          # Surpress v8 hooks... one wishes there was a better way to do this.
          { 'name': 'disable_depot_tools_selfupdate' },
          { 'name': 'landmines' },
          { 'name': 'clang_format_win' },
          { 'name': 'clang_format_mac' },
          { 'name': 'clang_format_linux' },
          { 'name': 'gcmole' },
          { 'name': 'jsfunfuzz' },
          { 'name': 'wasm_spec_tests' },
          { 'name': 'wasm_js' },
          { 'name': 'msan_chained_origins' },
          { 'name': 'msan_no_origins' },
          { 'name': 'win_toolchain' },
          { 'name': 'mac_toolchain' },
          { 'name': 'binutils' },
          { 'name': 'clang' },
          { 'name': 'lastchange' },
          { 'name': 'fuchsia_sdk' },
          { 'name': 'lld/mac' },
          { 'name': 'llvm-objdump' },
          { 'name': 'vpython_common' },
          { 'name': 'check_v8_header_includes' },
          { 'name': 'sysroot_arm' },
          { 'name': 'sysroot_arm64' },
          { 'name': 'sysroot_x86' },
          { 'name': 'sysroot_x64' },
        ],
        'custom_deps': {
            'v8/build': None,
            'v8/third_party/catapult': None,
            'v8/third_party/colorama/src': None,
            'v8/third_party/jinja2': None,
            'v8/third_party/markupsafe': None,
            'v8/testing/gmock': None,
            'v8/tools/swarming_client': None,
            'v8/tools/gyp': None,
            'v8/third_party/instrumented_libraries': None,
            'v8/third_party/android_tools': None,
            'v8/third_party/depot_tools': None,
            'v8/test/wasm-js': None,
            'v8/test/benchmarks/data': None,
            'v8/test/mozilla/data': None,
            'v8/third_party/icu': None,
            'v8/test/test262/data': None,
            'v8/test/test262/harness': None,
            'v8/tools/luci-go': None,
        }
    },
    {
        'url':
        'https://chromium.googlesource.com/chromium/src/build.git@082f11b29976c3be67dddd74bd75c6d1793201c7',
        'name': 'build',
    },
    {
        'url':
        'https://chromium.googlesource.com/chromium/src/buildtools.git@cf454b247c611167388742c7a31ef138a6031172',
        'name': 'buildtools',
    },
    {
        'url':
        'https://chromium.googlesource.com/chromium/src/tools/clang.git@c5d85f1e9d3a01e4de2ccf4dfaa7847653ae9121',
        'name': 'tools/clang',
    },
    {
        'url':
        'https://chromium.googlesource.com/chromium/src/third_party/jinja2.git@b41863e42637544c2941b574c7877d3e1f663e25',
        'name': 'third_party/jinja2',
    },
    {
        'url':
        'https://chromium.googlesource.com/chromium/src/third_party/markupsafe.git@8f45f5cfa0009d2a70589bcda0349b8cb2b72783',
        'name': 'third_party/markupsafe',
    },
]

hooks = [
  {
    # Ensure that the DEPS'd "depot_tools" has its self-update capability
    # disabled.
    'name': 'disable_depot_tools_selfupdate',
    'pattern': '.',
    'action': [
        'python',
        'third_party/depot_tools/update_depot_tools_toggle.py',
        '--disable',
    ],
  },
  # {
  #   # This clobbers when necessary (based on get_landmines.py). It must be the
  #   # first hook so that other things that get/generate into the output
  #   # directory will not subsequently be clobbered.
  #   'name': 'landmines',
  #   'pattern': '.',
  #   'action': [
  #       'python',
  #       'build/landmines.py',
  #       '--landmine-scripts',
  #       'tools/get_landmines.py',
  #   ],
  # },
  {
    'name': 'sysroot_arm',
    'pattern': '.',
    'condition': '(checkout_linux and checkout_arm)',
    'action': ['python', 'build/linux/sysroot_scripts/install-sysroot.py',
               '--arch=arm'],
  },
  {
    'name': 'sysroot_arm64',
    'pattern': '.',
    'condition': '(checkout_linux and checkout_arm64)',
    'action': ['python', 'build/linux/sysroot_scripts/install-sysroot.py',
               '--arch=arm64'],
  },
  {
    'name': 'sysroot_x86',
    'pattern': '.',
    'condition': '(checkout_linux and (checkout_x86 or checkout_x64))',
    'action': ['python', 'build/linux/sysroot_scripts/install-sysroot.py',
               '--arch=x86'],
  },
  {
    'name': 'sysroot_x64',
    'pattern': '.',
    'condition': 'checkout_linux and checkout_x64',
    'action': ['python', 'build/linux/sysroot_scripts/install-sysroot.py',
               '--arch=x64'],
  },
  {
    # Update the Windows toolchain if necessary.
    'name': 'win_toolchain',
    'pattern': '.',
    'condition': 'checkout_win',
    'action': ['python', 'build/vs_toolchain.py', 'update'],
  },
  {
    # Update the Mac toolchain if necessary.
    'name': 'mac_toolchain',
    'pattern': '.',
    'condition': 'checkout_mac',
    'action': ['python', 'build/mac_toolchain.py'],
  },
  # Pull binutils for linux, enabled debug fission for faster linking /
  # debugging when used with clang on Ubuntu Precise.
  # https://code.google.com/p/chromium/issues/detail?id=352046
  #{
  #  'name': 'binutils',
  #  'pattern': 'third_party/binutils',
  #  'condition': 'host_os == "linux"',
  #  'action': [
  #      'python',
  #      'v8/third_party/binutils/download.py',
  #  ],
  #},
  {
    # Note: On Win, this should run after win_toolchain, as it may use it.
    'name': 'clang',
    'pattern': '.',
    # clang not supported on aix
    'condition': 'host_os != "aix"',
    'action': ['python', 'tools/clang/scripts/update.py'],
  },
  {
    # Update LASTCHANGE.
    'name': 'lastchange',
    'pattern': '.',
    'action': ['python', 'build/util/lastchange.py',
               '-o', 'build/util/LASTCHANGE'],
  },
]
