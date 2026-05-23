### How to start

Run this command in the main app folder in wsl
Idk why but rust rover config isn't working

```shell
cargo run
```

### How to run tests

```shell
cargo test -p <nom-du-crate>
```

#### Spefic file tests

```shell
cargo test -p zoea-ecs -- storage::chunk
```

Pour supprimer les warnings lors des tests ajouter devant la commande `RUSTFLAGS="-Awarnings" `

### Tree View

```shell
tree -I 'target|*.md' /home/rcast/dev/exo-game
```


