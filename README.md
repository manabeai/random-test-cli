# random-test-cli

`random-test-cli` provides the `rt` command for generating sample test cases
from cp-ast-ecosystems share links.

```sh
rt 'https://manabeai.github.io/cp-ast-ecosystems/?state=...'
rt 'H4sIA...' --seed 42
rt open
rt completions bash > ~/.local/share/bash-completion/completions/rt
rt completions zsh > ~/.zfunc/_rt
rt completions fish > ~/.config/fish/completions/rt.fish
rt state.txt
rt update
```

The generator accepts compressed cp-ast share-link `state` values produced by
the web editor's copy-link button.
