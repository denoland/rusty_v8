# WARNING: This is not an automated tool! This is just some commands to copy and
# paste manually to upgrade V8.

export COMMITDATE=2020-01-30T00:37:18.000Z

git submodule foreach 'git remote rm upstream; true' && 
git -C build remote add upstream https://chromium.googlesource.com/chromium/src/build &&
git -C buildtools remote add upstream https://chromium.googlesource.com/chromium/src/buildtools &&
git submodule foreach 'git remote add upstream `git remote get-url origin`; true' &&
git submodule foreach 'git remote update' &&
git submodule foreach 'export SHA=`git log upstream/master -n1 --until=$COMMITDATE --pretty=%H` && git merge $SHA -m "Merge commit $SHA from `git remote get-url upstream`"'

export BRANCH=20200130_rusty_v8
git -C build push git@github.com:denoland/chromium_build HEAD:refs/heads/$BRANCH
git -C buildtools push git@github.com:denoland/chromium_buildtools HEAD:refs/heads/$BRANCH
