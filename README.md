# [RozsdásFilc](https://codeberg.org/jark/rsfilc): [`E-Kréta`](https://www.e-kreta.hu/) console client in [Rust](https://rust-lang.org)

> `E-Kréta` is an awful Hungarian electronic school administration system

> [Magyar leírás](README.hu.md)

[![dependency status](https://deps.rs/repo/codeberg/jark/rsfilc/status.svg)](https://deps.rs/repo/codeberg/jark/rsfilc)

## Installation

-   EZ mode: grab a prebuilt binary from [releases](https://codeberg.org/jark/rsfilc/releases/latest)

if not available for your platform ([file an issue](https://codeberg.org/jark/rsfilc/issues/new)), not a preferred method or feels a bit outdated:

-   [Rust](https://rustup.rs)
-   `cargo install --locked rsfilc`
>   for latest, beta builds: `cargo install --locked --git "https://codeberg.org/jark/rsfilc"`

### Shell completions:

   <details>
   <summary>Bash</summary>

> Add this to the <ins>**end**</ins> of your config file (usually `~/.bashrc`):
>
> ```sh
> eval "$(rsfilc completions bash)"
> ```

   </details>

   <details>
   <summary>Zsh</summary>

> Add this to the <ins>**end**</ins> of your config file (usually `~/.zshrc`):
>
> ```sh
> eval "$(rsfilc completions zsh)"
> ```
>
> For completions to work, the above line must be added _after_ `compcompletions` is
> called. You may have to rebuild your completions cache by running
> `rm ~/.zcompdump*; compcompletions`.

   </details>

   <details>
   <summary>Fish</summary>

> Add this to the <ins>**end**</ins> of your config file (usually `~/.config/fish/config.fish`):
>
> ```fish
> rsfilc completions fish | source
> ```

   </details>

   <details>
   <summary>PowerShell</summary>

> Add this to the <ins>**end**</ins> of your config file (find it by running `echo $profile` in PowerShell):
>
> ```powershell
> Invoke-Expression (& { (rsfilc completions powershell | Out-String) })
> ```

   </details>

   <details>
   <summary>Elvish</summary>

> Add this to the <ins>**end**</ins> of your config file (usually `~/.elvish/rc.elv`):
>
> ```sh
> eval (rsfilc completions elvish | slurp)
> ```
>
> **Note**
> RsFilc only supports elvish v0.18.0 and above.

   </details>

## Usage

general help: `rsfilc --help`  
log in to an account: `rsfilc user --login <USER_ID>`

### useful stuff

- when in doubt, be sure to check `rsfilc --help` first
- if you'd like to have instant replies, only loading cached data, not caring about latest changes on the server, you shall try setting the environment variable `NO_NET` to `1`, eg. on linux: `NO_NET=1 rsfilc timetable`
- if you feel like refreshing your cache, you'd do (again on linux): `NO_CACHE=1 rsfilc absences`, but don't forget `rsfilc user --cache-dir` either
- if your lessons are over for a day, and say: `rsfilc (timetable|tt)`, the next day with lessons will be shown by default, to show today you could: `rsfilc tt +0` or `rsfilc tt -- -0`

![demo](./assets/demo.gif "using rsfilc")

## Features

### renaming

like: teachers, subjects, ...  
check out the config path with `rsfilc --config-path` and edit `rename` to your liking  
you may wanna check out [example config](./assets/example-config.toml)
if you don't feel like renaming at all at a specific moment, use `NO_RENAME=1` (*note*: this won't apply to already cached data)

### cross-platform: not tested thoroughly but should run on

-   linux
-   windows
-   macOS
-   android via Termux
-   everything else that Rust supports

## CLI

-   [x] basic usage of API
-   [x] filtering what to show
-   [x] multi-user feature
-   [x] somehow rendering `html` that messages return
-   [x] render `html` messages with `w3m` or `lynx` if possible
-   [x] shell completions: [bash, zsh, fish, elvish, powershell]
-   [x] logger: `fern` maybe
-   [ ] helpful crashes
-   [x] caching everything so that life remains enjoyable
    -   [x] token
    -   [x] timetable
    -   [x] evals
    -   [x] user info
    -   [x] absences
    -   [x] announced tests
    -   [x] messages
    -   [x] note messages
-   [x] encoding passwords
-   [ ] sending messages
-   [ ] class averages
-   [ ] fetching administrational processes
-   [ ] starting new administrational processes

## Acknowledgements

-   [dependencies used](./Cargo.toml) (although many of them wouldn't be necessary with a proper API, this is **not** the case with e-kréta.)
-   See [ekreta-rs](https://codeberg.org/jark/ekreta-rs), which provides the API client for E-Kréta
-   Shell completions section got from [zoxide](https://github.com/ajeetdsouza/zoxide)
