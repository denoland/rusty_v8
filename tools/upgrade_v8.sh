# WARNING: This is not an automated tool! This is just some commands to copy and
# paste manually to upgrade V8.

# SHAs come from v8/DEPS.

git submodule update --recursive -f
git submodule foreach --recursive 'git remote --get-url upstream && git remote rm upstream'
git -C build remote add upstream https://chromium.googlesource.com/chromium/src/build
git -C buildtools remote add upstream https://chromium.googlesource.com/chromium/src/buildtools
git submodule foreach --recursive 'git remote --get-url upstream || git remote add upstream $(git remote get-url origin)'
git submodule foreach --recursive 'git remote update'

SHA=cab90cbdaaf4444d67aef6ce3cef09fc5fdeb560 eval '(cd base/trace_event/common; git checkout $SHA)'
SHA=81d656878ec611cb0b42d52c82e9dae93920d9ba eval '(cd third_party/icu; git checkout $SHA)'
SHA=11b6b3e5971d760bd2d310f77643f55a818a6d25 eval '(cd third_party/jinja2; git checkout $SHA)'
SHA=0944e71f4b2cb9a871bcbe353f95e889b64a611a eval '(cd third_party/markupsafe; git checkout $SHA)'
SHA=09490503d0f201b81e03f5ca0ab8ba8ee76d4a8e eval '(cd third_party/zlib; git checkout $SHA)'
SHA=a387faa2a6741f565e45d78804a49a0e55de5909 eval '(cd tools/clang; git checkout $SHA)'

SHA=77edba11e25386aa719d4f08c3ce2d8c4f868c15 eval '(cd build; git merge $SHA -m "Merge commit $SHA from $(git remote get-url upstream)")'

(
  cd buildtools
  SHA=5dbd89c9d9c0b0ff47cefdc2bc421b8c9a1c5a21 eval 'git merge $SHA -m "Merge commit $SHA from $(git remote get-url upstream)"'
  SHA=8fa87946779682841e21e2da977eccfb6cb3bded eval '(cd third_party/libc++/trunk; git checkout $SHA)'
  SHA=d0f33885a2ffa7d5af74af6065b60eb48e3c70f5 eval '(cd third_party/libc++abi/trunk; git checkout $SHA)'
  git add 'third_party/libc++' 'third_party/libc++abi'
  git commit --amend -C HEAD
)

export BRANCH=$(date +'%Y%m%d')_rusty_v8
git -C build push git@github.com:denoland/chromium_build HEAD:refs/heads/$BRANCH
git -C buildtools push git@github.com:denoland/chromium_buildtools HEAD:refs/heads/$BRANCH

git -C build push git@github.com:denoland/chromium_build upstream/master:refs/heads/upstream
git -C buildtools push git@github.com:denoland/chromium_buildtools upstream/master:refs/heads/upstream
