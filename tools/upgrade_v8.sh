# WARNING: This is not an automated tool! This is just some commands to copy and
# paste manually to upgrade V8.

export BRANCH=20210101_rusty_v8
export COMMITDATE=2020-11-12T08:56:11.000Z

git submodule update -f
git submodule foreach 'git remote rm upstream; true' &&
git -C build remote add upstream https://chromium.googlesource.com/chromium/src/build &&
git -C buildtools remote add upstream https://chromium.googlesource.com/chromium/src/buildtools &&
git submodule foreach '[ -f ./include/v8.h ] || (git remote add upstream `git remote get-url origin`; true)' &&
git submodule foreach '[ -f ./include/v8.h ] || git remote update' &&
git submodule foreach '[ -f ./include/v8.h ] || (export SHA=`git log upstream/master -n1 --until=$COMMITDATE --pretty=%H` && git merge $SHA -m "Merge commit $SHA from `git remote get-url upstream`")'

git -C build push git@github.com:denoland/chromium_build HEAD:refs/heads/$BRANCH
git -C buildtools push git@github.com:denoland/chromium_buildtools HEAD:refs/heads/$BRANCH

git -C build push git@github.com:denoland/chromium_build upstream/master:refs/heads/upstream
git -C buildtools push git@github.com:denoland/chromium_buildtools upstream/master:refs/heads/upstream
