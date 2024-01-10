# `simulo_bevy`

Welcome to Simulo, also known as Bevyulo, a 2D physics sandbox in Rust using the Bevy game engine.

> [!WARNING]
> This project is still in early development. Expect bugs and missing features.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/)
- If you're using Linux, you'll need [these dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md) as well as `g++-12`

### Installation

1. Clone the repo

   ```sh
   git clone https://github.com/Carroted/simulo_bevy.git
   cd simulo_bevy
   git switch liquidfun
   ```

> [!TIP]
> If you have SSH enabled on your GitHub account &mdash; which you should &mdash; you can use `git@github.com:Carroted/simulo_bevy.git` instead of the HTTPS link.

2. Run the project

   ```sh
   cargo run --release
   ```

> [!NOTE]
> If you're running `simulo_bevy` to develop it, don't use `--release` as it will slow down the compilation time and make debugging harder. `--release` is only for when you want to run the game at full speed.

## License

[GNU GPLv3](https://choosealicense.com/licenses/gpl-3.0/)

## Credits

- [Bevy](https://bevyengine.org/)
- [`bevy_liquidfun`](https://github.com/mmatvein/bevy_liquidfun) (we use [this fork](https://github.com/Carroted/bevy_liquidfun))
- more stuff, will make a proper list later
