#!/bin/bash

set -e

# このcargo_fmt.shがあるdirectoryのpath
CURRENT_DIR=$(echo $(cd $(dirname $0) && pwd))

# repositoryのroot
PROJECT_ROOT="${CURRENT_DIR}/.."

# repositoryのrootへ移動
cd $PROJECT_ROOT

# clippyが入っていない場合は入れる
if ! $(rustup component list | grep "clippy" | grep "installed" > /dev/null); then
	rustup component add clippy
fi

if ! $(rustup component list | grep "rustfmt" | grep "installed" > /dev/null); then
	rustup component add rustfmt
fi

# clippyで上書き保存する（このリポジトリの対象ファイルをGit Commitしないとエラーになる）
cargo clippy --no-deps --fix

# formatする
cargo fmt
