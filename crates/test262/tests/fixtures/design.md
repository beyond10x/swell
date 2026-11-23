# Fixture design for the tests of swell-test262

Written for these tests. It holds one `swell-slices` block with every line kind the runner reads.

```swell-slices
order delivery 1,2,3,4,7,10,14,21
dir test/harness/ 1
dir test/language/ 2
dir test/built-ins/ 3
dir test/built-ins/Number/ 4
dir test/built-ins/Atomics/ 14
dir test/intl402/ 21
feature generators 7
feature Atomics 14
feature intl-normative-optional 21
flag async 10
flag CanBlockIsTrue 14
exclude decorators stage3
exclude-slice 21 intl402
exclude-include agentHelper.js agents
```

Text after the block is not read.

```
dir test/ 99
```
