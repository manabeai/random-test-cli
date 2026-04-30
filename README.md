# random-test-cli

`random-test-cli` provides the `rt` command for generating sample test cases
from cp-ast-ecosystems share links.

```sh
rt 'https://manabeai.github.io/cp-ast-ecosystems/?state=...'
rt '%7B%22schema_version%22%3A1%2C...%7D' --seed 42
rt open
rt state.txt
```

The generator accepts URL-encoded JSON `state` values.
