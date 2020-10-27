# cargo-simple-bundler

クレートの必要なモジュールだけを一つのファイルにまとめる作業を自動化します．
単一ファイルの提出のみを受け付けるオンラインジャッジのために利用されることを想定しています．

## インストール

```
cargo install --git https://github.com/kuretchi/cargo-simple-bundler
```

## 使い方

例として，[ac-library-rs](https://github.com/rust-lang-ja/ac-library-rs) を利用する次のような
`main.rs` を書いたとします．

```rust
use ac_library_rs::LazySegtree;
fn main() {
    // LazySegtree を使うコード
}
```

ここで次のように実行します．

```
cargo simple-bundler --manifest-path /path/to/ac-library-rs/Cargo.toml -e main.rs \
    --remove-doc-comments --remove-test-modules \
    >>main.rs
```

これで ac-library-rs のうち `ac_library_rs::LazySegtree` の利用に必要な部分のみが，
そのままコンパイルできる状態で `main.rs` に追記されます．

## 詳しい説明

外部ファイルを参照する `mod hoge;` のようなモジュール宣言を
`mod hoge { /* hoge.rs の中身 */ }` に置き換える操作を再帰的に行うことで，
元のコード群と等価な一つの大きなファイルが得られます．
`--entry-file-path` を指定しない場合はこの操作 (+α) のみを行います．

`--entry-file-path` でファイル (以降，これをエントリファイルと呼びます) を指定した場合，
まずエントリファイル中の `use crate_name` (`crate_name` はクレートの名前) から始まる
`use` 宣言をすべて見て，エントリファイルがクレート内のどのモジュールに特に依存しているかを調べます．
その後，依存先のモジュールが依存するモジュールを同様の方法で調べます．
以上を繰り返してモジュール同士の依存関係のグラフの連結成分を取り出し，
それに含まれないモジュールをうまく削除した状態で提示します．

### 不要なモジュールの削除

あるモジュール `a` に依存していると判定された場合，`a` 以下のすべてのコードは最終結果に含まれます．
`a` の祖先モジュール (`a` を定義しているモジュール，そのモジュールを定義しているモジュール，…)
は編集が加えられ最終結果に含まれます．

例：

```rust
// lib.rs
mod a;
mod b;
```

ここでモジュール `b` が不要であると判定された場合は，`mod b;` 宣言 (とその下のコード) は
最終結果に含まれません．

### 最終結果の出力

結合されたコードは `mod crate_name { /* ここ */ }` に書き込まれた状態で出力されます．
その際に，コード中の `crate` キーワードは `crate::crate_name` に書き換えられます．

最終結果からさらに不要なコードを削除するための次のようなフラグがあります．

`--remove-doc-comments`: ドキュメンテーションコメントを削除します．
`--remove-test-modules`: `#[cfg(test)]` 属性が付いたインラインモジュールを削除します．

### 依存モジュールの判定

(`pub` や `pub(restricted)` でない) `use` 宣言のうち，
パスが `crate`，`super`，`self` から始まるもののみを認識します．
構造体等の，モジュール以外の公開アイテムへの依存は，
そのアイテムが定義されているモジュールへの依存と見なします．

例：

```rust
// lib.rs
pub mod a;
pub mod b;
pub mod c;
```

```rust
// a.rs
use super::c::C;
pub struct A(C);
```

```rust
// b.rs
pub struct B;
```

```rust
// c.rs
pub struct C;
```

ここでエントリファイルが `use crate_name::a::A;` を含むとします．
アルゴリズムはまず {`crate::a::A`} への依存があるという状態から開始し，
最終的に {`crate::a`, `crate::c`} への依存があると結論します．

### `pub use` 宣言による再公開

`pub use` 宣言は，次のように `self` から始まる (`self` はなくてもよい)
一段階の単純なものに限り追跡されます．

```rust
// lib.rs
pub use self::a::*;
mod a;
```

```rust
// a.rs
pub struct A;
```

ここで `crate::A` のようなパスは `crate::a::A` を指していると正しく認識されます．
また，このモジュール `a` が削除される場合，対応する `pub use` 宣言も削除されます．

`pub use` 宣言は，先述した `use` 宣言による依存モジュールの判定の対象からは除外されます．
この挙動を変更したい場合は，次のようにダミーの宣言を付記することで対処できます．

```rust
pub use self::a::*;
#[cfg(any())]
use self::a::*;
```

## 注意

このツールの利用を想定していない一般的なクレートに対して動作することは目標にしておらず，
うまく動作するには (以下で述べるものに限らず) いくつかの制限があります．

### マクロ

`macro_use!` による宣言的マクロの利用は追跡されませんが，
マクロが定義されているモジュールへの依存を `use` 宣言で明示することで対処できます．

公開アイテム等が次のようにマクロで宣言される場合，正しく認識されません．

```rust
macro_use! def {
    () => {
        pub struct A;
    };
}
def! {}
```

この場合は，次のようにダミーの宣言を外側に置くことで対処できます．

```rust
#[cfg(any())]
pub struct A;
```

マクロ内の `crate` キーワードは認識されないため，手動で書き換える必要があります．

### その他の未対応機能等

インラインモジュール，複雑な `pub use` 宣言，リネーム (`use foo as bar;`)，
`pub(restricted)`，`path` 属性，`mod.rs` (あるいは Rust 2015 edition のモジュールシステム) など

## ライセンス

[MIT License](./LICENSE-MIT) or [Apache License 2.0](./LICENSE-APACHE)
