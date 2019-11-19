solutions = [
    {
        'url': 'https://chromium.googlesource.com/v8/v8.git@7.9.317.12',
        'name': 'v8',
        'deps_file': 'DEPS',
        'custom_deps': {
            #'v8/build': None,
            'v8/third_party/catapult': None,
            'v8/third_party/colorama/src': None,
            'v8/third_party/jinja2': None,
            'v8/third_party/markupsafe': None,
            'v8/testing/gmock': None,
            'v8/tools/swarming_client': None,
            'v8/tools/gyp': None,
            'v8/third_party/instrumented_libraries': None,
            'v8/third_party/android_tools': None,
            #'v8/third_party/depot_tools': None,
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
