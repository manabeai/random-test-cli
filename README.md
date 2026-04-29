# random-test-cli

`random-test-cli` provides the `rt` command for generating sample test cases
from cp-ast-ecosystems share links.

```sh
rt 'https://manabeai.github.io/cp-ast-ecosystems/?state=...'
rt 'v2....' --seed 42
rt browse
```

The generator accepts both the current compressed `v2.` share state and legacy
base64 state values.
