# Key Evolving Signatures

`kes` is a pure rust implementation of Key Evolving Signatures, following the paper
from ["Composition and Efficiency Tradeoffs for Forward-Secure Digital Signatures"](https://eprint.iacr.org/2001/034)
by Malkin, Micciancio and Miner. In particular, we implement the "sum" composition, from Section
3.1. As a depth zero signature algorithm we use Ed25519 using the `strict` verification criteria from
[`ed25519_dalek`](https://github.com/dalek-cryptography/ed25519-dalek), which is the same as currently
used in [libsodium](https://github.com/jedisct1/libsodium).

This library defines macros to generate KES algorithms with different depths. We currently expose KES
algorithms up to depth 7. However, if you require a higher depth key, feel free to open an
issue/PR.

## Library usage
This library exposes `SumXKes` for `X` in [2,7]. A KES algorithm with depth `X` can evolve the key
`2^X`. When a secret key is evolved, the old seed is overwritten with zeroes.

```rust
use pallas_crypto::kes::summed_kes::Sum6Kes;
use pallas_crypto::kes::traits::{KesSig, KesSk};

fn main() {
    let (mut skey, pkey) = Sum6Kes::keygen(&mut [0u8; 32]);
    let dummy_message = b"tilin";
    let sigma = skey.sign(0, dummy_message);

    assert!(sigma.verify(0, &pkey, dummy_message).is_ok());

    // Key can be updated 63 times
    for i in 0..63 {
        assert!(skey.update(i).is_ok());
    }
}
```

**Note** Cardano uses currently **Sum6Kes**.

## Command-Line

`kes` comes with an optional command-line interface for Linux. The command-line is self explanatory by using `--help` on various commands and sub-commands.
Build with flag: `--features="kes_cli"` to have

### How to randomly generate a 32-byte valid secret seed (<strong>seed.prv</strong>)

```console
$ cargo run --features="kes_cli" --quiet -- generate-seed ; echo
bf410498bcb54308b2f9483a488430610fb40e4dd7d84baa1bbb35174231b0e0
$ cargo run --features="kes_cli" --quiet -- generate-seed ; echo
76a0e64fb116d8dedde3d2d3de8c14a0c96d18b05ec7fa4e7e7c409df7985598
```

### How to randomly generate a 612-byte valid signing key (<strong>sk.prv</strong>)

```console
$ cargo run --features="kes_cli" --quiet -- generate-sk ; echo
2ad497b6916eeee80ad9684c7e84ecaa5a29b41b3fa9568c274f522701e3243bcb19f7e677655ef84055a8f18f5ac735e0740a5910dc13fd50858eee17bc8195b520a6ff1b26cb5981ede21e4424b95d08f5cb5ec778fad20ef0d2554fe494d89ee6bcdc03ac6cace7d3c090ec416b706792f6249d19d94c2e4ec6acaa38ebd7ba374ea697845d98bb514749e6c58ff4cd61047be72327ba531c1aef4e63329a2bf6529b0de0dfb4f03ff7ea33f3bc40b9153fccea8ee1a8da4d7d476ec58e889fca891f462876a41dfc2b8472a1e60d52b73598c73cfe25286bb90e01902551dab4c21f4b14a90b482e7d80366c75f97c7dc41b7fc57b76d404c2760d62a2cdd076099c58b8b09f774ea303471e0fdbeee594f8bc91fcf33290b1306cc369483147a6c79400feaa4ce0b71a48120f21962433c8850e9a5c3029817b829a84804ef17d293bc69ff7867e1de175bf2e138236f5ab6dd5825ce239e69cc34694e36a4dbe7b1821564e21bd3ffac2ec44eb05b1bc8ad5a9aed4142a463ae46970d25dfe7ac747f0924f777993e7bd3a5556ef6584d03535315be4c4a9d691159bb0a1355160d00798c5d93c8daffee3eb2b067cb99a20d5adb4f0dea9254fbd3c7e4fa53f73bf1faa689698ff1847ac3486922c362556747b487ee67d55fb34206f5f8bd617395fe6bed77c7cc98f1b0d848854411beab9876c6e15a23d1ca5f93d445d9a57bc7a5e9e870945f1b4e9b22e242c87e585930b41ed63f5ace2ed3f1fc4616161d7001711bc47785648c7608199edfc5a11a129cbe94757d1e4c5e8f7d8c06d391ad58ab1f8ddd9380f0646b702830a89a2b2d6592b4e26b4fa5157c200000000
$ cargo run --features="kes_cli" --quiet -- generate-sk ; echo
e3fb7b7a2113941ae4f0e0aa50f08d25afe0ddd31c17054e71d2d1189d1136aa10e30bdb561bb2a98f2029b735ecbaec47a99673e872a58447d9dad7039c826635448955c61483f61d2c1dd056c9bdddd06ad17e8a024e6f4a45056496884d338772f4bf6c23b645b1080859a45e61153b238cd6b648c275497dbae1980288559cb757a26a0d23b7c53784549634bedbeb4c2ab1b4c055f251621418ae7ea5cf89d9651a5ea9c02e0325eeeea398c2b509fb612358991db7ac6ecfcb3e39a107cd8302e6c8749b9f61822b765c28a9fa4b68fd1ac7e4bdc98f6e1d3ecbc451ca4d6e0225138ba366fc2c9b4ad9fb3ef6fdc64f3203d374912b3a88e3ab3a1121c5c73af5b1425641502a4eeeb4ce481a21b34dbb6680936651f72d220c098296169b46c5de4e14010db5ac46e55d440d69882a108f39360b3e1f7e43bde82299122b7a6a1ad536a720b2893e8db1ed3f8b47330faaa8c7d3b467169276e17ec565bff33a32646f9a557a3f97fd38456da7bfe33949212193871006e44b26c52444f29ecf82e19552584b2e0d9e29bdc8d2dc8b87f98ea3bccbad54770cb575a36322cb2799c8270fbb86c10ba8f7a0eeabf02194880c6fae8da0c6a155c6d686d390b12aa28303d682648c62805782d5ef0a7f9a4eb5278435b9d8e55b23af64f3da3ac210c15dd387542901fe63f9135347bee6caddf93acdccc498bd11d36b7118c866218dc13829044cd5e2dea572138acc37a811e1b8b32c251dc1ca2566f133ef8d1d06bd41a2466e8cf6ce37e3a2d05697bb9e7ed2a10b3d88619e9b76f2f81b6c8360bb743606a2ad346fcdb759db7580ffeb0f94a7c2771e8cd0e40c00000000
```

### How to derive a 612-byte signing key from a 32-byte secret seed (<strong>sk.prv</strong>)

```console
$ cargo run --features="kes_cli" --quiet -- generate-seed | cargo run --features="kes_cli" --quiet -- derive-sk -f - ; echo
75dfe9d37f2ebb676b279cdc8c3d8a3bf4c1b495704ecc16484436814c25ca5441eede8b68c46555ae57fb9ab535e4fcf4666f93ca7aaae4f5cab5f007eba98757361eeeff919f8b48a4c119382587e4d25e817a0b38f223d2b997991a19c4b237032e010758685d1a644533377f86b3582ff9c139c5278d518b7f015278ba669b4a6ff891d38c6216cb1c94a0b67ced2202e6d6083c10bcf7097d0256623fa22e5ce514c5c2fba6d91df8e01de45ea95306599b40d41c092c85985f64125712b7b90f74aa94bd8d6caea37dfffc8e8dccd5a7c2c7896eb338329fdd6692eb609258324e4f3c4f2459ab5fc3d9f25c338509e671377a23397871d8cea94b362c1c6bd5bf41347eb02a7a96424dcfa27f70d743a3acb2fc912179629d652ba3815a246838c0904793580e7f8e887c99d185d4968cc3bfd7879b50f6b054e1453d956fae52ea0570f2ecee74c845d82db8b0bca5941976e3923ab5020186e2da34986f9cba55b362dc72a7cc3417931bc8d6a0605b684df9ab9b56d46d0fe754ba41edd4723de271708606fb70649d594006591f2c616c271eecdcdd8d49cb98f0e0eb3297f01088e5018de45019d76dbaaf7ae5c85c0bb0a73c0ca59e4aac0d60dcebafbee3c61b53fda020133b66aa91efd3d5271af922b0dbe55db891f4fca7d9e75ddd176623ab6b95d4231903d4ca14472fe54d78593b5314c466c275b4b826852479c2c2f3bbbf5c1d39f979271626b1ab5830d82f0d9332f1b5a7188432896adb9843bfea840531683c17e604b0782deb487b7e38c7eb46c6a20652ae6affe333c4c7bef9bc30bdf7cc35b12e2c42eab7a37b8684fba5d8c28c0f43a57300000000

$ echo -n 7fe54ac4449ef108b4717620b36085f300de9758decd6ad240b24b37d3f3dfc5 | cargo run --features="kes_cli" --quiet -- derive-sk --file -; echo
60cb0d00b11878c25d9b906607bdad7dd79d732484bfaf48b043eb4c17bbd09d44e5e3031be8b67c0d25247c35d767fa5ad5266e3a54fb851f16e0cf8e84eabd9e7805d1c6f2e8f6a7546ba288952cd8b2f27532ed28ba0205aab0b2a0ce3b3fc4f63b08c1c325589d6faed23b315802a6cef82f68352a2116ff2755956258467a4b5aa8f157a6c683c8a4a9c1ac2b073eb8ea1b919e745de5d7d51a2f18c6e11bb6a2b5b9e3837cd0d5848a5db4aee7d2fdf7ab734833fbfe8c1470128fd5707b85710dddb4ebdc49eff82a74238c2e64a1e16585cb52a80c5fae6c3c592e3c9a06b0627b41b9bc7b6c514542c8ec5f85224798e39ea27f2c34c72ce925489eb8212f14e67fd788f49abfb950bd6d9c07ab3783ce86c9937dfa4fd333bd1f6e82478cfd5ad30773f4096b314e9bcfff8770052b2fe58547d03f6cd7848fe24f1efeeec17099b26058ac94fe54268472838da00da4154694380bdc8c6f9ee43103cdf0d93d54eb615e05a8ebeb9a95876b818e2f5522f8675cecead549dd13acaca314c9f01a556197bf93ece5b544a1ee136bc116412fd1ca51fe0874c2d8b4794e754cb8d73e8bfe7d21acab20b0707f2be54cd8f99bc568ccccfeebe3928810cc8840f62b3785f578a51f37f0756fe3ca43579e4d049ce9d5754e4e4ada908b0e063f3e52e24d1bee78f7cfab25b1bb867d562c93d53b0143ecc3b79924ea40087b687ede4d15a66c46d44a29002ecd9da6995f1d0b4e0a7be396f29e544b9a762b52d93e17818e396bd5192b9fae26f34dfdda3dbd7e666b0be11dce71cc083279b1e66d9913bd97c576143d3994a4374367380f8c8c784f62050622307000000000

$ echo -n 7fe54ac4449ef108b4717620b36085f300de9758decd6ad240b24b37d3f3dfc5 > seed
$ cargo run --features="kes_cli" --quiet -- derive-sk --file seed; echo
60cb0d00b11878c25d9b906607bdad7dd79d732484bfaf48b043eb4c17bbd09d44e5e3031be8b67c0d25247c35d767fa5ad5266e3a54fb851f16e0cf8e84eabd9e7805d1c6f2e8f6a7546ba288952cd8b2f27532ed28ba0205aab0b2a0ce3b3fc4f63b08c1c325589d6faed23b315802a6cef82f68352a2116ff2755956258467a4b5aa8f157a6c683c8a4a9c1ac2b073eb8ea1b919e745de5d7d51a2f18c6e11bb6a2b5b9e3837cd0d5848a5db4aee7d2fdf7ab734833fbfe8c1470128fd5707b85710dddb4ebdc49eff82a74238c2e64a1e16585cb52a80c5fae6c3c592e3c9a06b0627b41b9bc7b6c514542c8ec5f85224798e39ea27f2c34c72ce925489eb8212f14e67fd788f49abfb950bd6d9c07ab3783ce86c9937dfa4fd333bd1f6e82478cfd5ad30773f4096b314e9bcfff8770052b2fe58547d03f6cd7848fe24f1efeeec17099b26058ac94fe54268472838da00da4154694380bdc8c6f9ee43103cdf0d93d54eb615e05a8ebeb9a95876b818e2f5522f8675cecead549dd13acaca314c9f01a556197bf93ece5b544a1ee136bc116412fd1ca51fe0874c2d8b4794e754cb8d73e8bfe7d21acab20b0707f2be54cd8f99bc568ccccfeebe3928810cc8840f62b3785f578a51f37f0756fe3ca43579e4d049ce9d5754e4e4ada908b0e063f3e52e24d1bee78f7cfab25b1bb867d562c93d53b0143ecc3b79924ea40087b687ede4d15a66c46d44a29002ecd9da6995f1d0b4e0a7be396f29e544b9a762b52d93e17818e396bd5192b9fae26f34dfdda3dbd7e666b0be11dce71cc083279b1e66d9913bd97c576143d3994a4374367380f8c8c784f62050622307000000000
```

### How to derive a 32-byte public key from a 612-byte signing key (<strong>pk.pub</strong>)

```console
$ echo -n 7fe54ac4449ef108b4717620b36085f300de9758decd6ad240b24b37d3f3dfc5 | cargo run --features="kes_cli" --quiet -- derive-sk --file - > sk
$ cargo run --features="kes_cli" --quiet -- derive-pk --file sk
4b31d9f3147ed2407b723e3903e33be5bdb0f33486a81684aeb2537b23c4cf2a

$ echo -n 7fe54ac4449ef108b4717620b36085f300de9758decd6ad240b24b37d3f3dfc5 | cargo run --features="kes_cli" --quiet -- derive-sk --file - | cargo run --features="kes_cli" --quiet -- derive-pk --file -
4b31d9f3147ed2407b723e3903e33be5bdb0f33486a81684aeb2537b23c4cf2a
```

### How to get period from a 612-byte signing key (<strong>period</strong>)
```console
$ echo -n 7fe54ac4449ef108b4717620b36085f300de9758decd6ad240b24b37d3f3dfc5 | cargo run --features="kes_cli" --quiet -- derive-sk --file - > sk
$ cargo run --features="kes_cli" --quiet -- period --file sk ;echo
0
```

### How to sign message using a 612-byte signing key (<strong>signature</strong>) and verify signature using the corresponding public key
```console
$ echo -n 7fe54ac4449ef108b4717620b36085f300de9758decd6ad240b24b37d3f3dfc5 | cargo run --features="kes_cli" --quiet -- derive-sk --file - > sk
$ cargo run --features="kes_cli" --quiet -- derive-pk --file sk > pk
$ echo -n "msg" | cargo run --features="kes_cli" --quiet -- sign -f sk
50d9074cdaf645b0e0aae42ce1c2cee585d109e9fcf394674b73c85ce0a1dc1da45b8e1d31b2195cd9db08b8f4650d36db6f3d67570ff04befa9feded6ef380f9e7805d1c6f2e8f6a7546ba288952cd8b2f27532ed28ba0205aab0b2a0ce3b3fc4f63b08c1c325589d6faed23b315802a6cef82f68352a2116ff2755956258461bb6a2b5b9e3837cd0d5848a5db4aee7d2fdf7ab734833fbfe8c1470128fd5707b85710dddb4ebdc49eff82a74238c2e64a1e16585cb52a80c5fae6c3c592e3cb8212f14e67fd788f49abfb950bd6d9c07ab3783ce86c9937dfa4fd333bd1f6e82478cfd5ad30773f4096b314e9bcfff8770052b2fe58547d03f6cd7848fe24f03cdf0d93d54eb615e05a8ebeb9a95876b818e2f5522f8675cecead549dd13acaca314c9f01a556197bf93ece5b544a1ee136bc116412fd1ca51fe0874c2d8b410cc8840f62b3785f578a51f37f0756fe3ca43579e4d049ce9d5754e4e4ada908b0e063f3e52e24d1bee78f7cfab25b1bb867d562c93d53b0143ecc3b79924ea9a762b52d93e17818e396bd5192b9fae26f34dfdda3dbd7e666b0be11dce71cc083279b1e66d9913bd97c576143d3994a4374367380f8c8c784f620506223070
$ echo -n "msg" | cargo run --features="kes_cli" --quiet -- sign -f sk > sig
$ echo -n "msg" | cargo run --features="kes_cli" --quiet -- verify -f pk -p 0 -s sig
OK
$ echo -n "msg" | cargo run --features="kes_cli" --quiet -- verify -f pk -p 1 -s sig
Fail
$ echo "msg" | cargo run --features="kes_cli" --quiet -- verify -f pk -p 0 -s sig
Fail
```

### How to update 612-byte signing key (<strong>sk.prv</strong>)

```console
$ echo -n 7fe54ac4449ef108b4717620b36085f300de9758decd6ad240b24b37d3f3dfc5 | cargo run --features="kes_cli" --quiet -- derive-sk --file - > sk
$ cargo run --features="kes_cli" --quiet -- derive-pk --file sk > pk
$ echo -n "msg" | cargo run --features="kes_cli" --quiet -- sign -f sk
$ cargo run --features="kes_cli" --quiet -- update -f sk
44e5e3031be8b67c0d25247c35d767fa5ad5266e3a54fb851f16e0cf8e84eabd00000000000000000000000000000000000000000000000000000000000000009e7805d1c6f2e8f6a7546ba288952cd8b2f27532ed28ba0205aab0b2a0ce3b3fc4f63b08c1c325589d6faed23b315802a6cef82f68352a2116ff2755956258467a4b5aa8f157a6c683c8a4a9c1ac2b073eb8ea1b919e745de5d7d51a2f18c6e11bb6a2b5b9e3837cd0d5848a5db4aee7d2fdf7ab734833fbfe8c1470128fd5707b85710dddb4ebdc49eff82a74238c2e64a1e16585cb52a80c5fae6c3c592e3c9a06b0627b41b9bc7b6c514542c8ec5f85224798e39ea27f2c34c72ce925489eb8212f14e67fd788f49abfb950bd6d9c07ab3783ce86c9937dfa4fd333bd1f6e82478cfd5ad30773f4096b314e9bcfff8770052b2fe58547d03f6cd7848fe24f1efeeec17099b26058ac94fe54268472838da00da4154694380bdc8c6f9ee43103cdf0d93d54eb615e05a8ebeb9a95876b818e2f5522f8675cecead549dd13acaca314c9f01a556197bf93ece5b544a1ee136bc116412fd1ca51fe0874c2d8b4794e754cb8d73e8bfe7d21acab20b0707f2be54cd8f99bc568ccccfeebe3928810cc8840f62b3785f578a51f37f0756fe3ca43579e4d049ce9d5754e4e4ada908b0e063f3e52e24d1bee78f7cfab25b1bb867d562c93d53b0143ecc3b79924ea40087b687ede4d15a66c46d44a29002ecd9da6995f1d0b4e0a7be396f29e544b9a762b52d93e17818e396bd5192b9fae26f34dfdda3dbd7e666b0be11dce71cc083279b1e66d9913bd97c576143d3994a4374367380f8c8c784f62050622307000000001
$ cargo run --features="kes_cli" --quiet -- update -f sk > sk1
$ echo -n "msg" | cargo run --features="kes_cli" --quiet -- sign -f sk1 > sig1
$ echo -n "msg" | cargo run --features="kes_cli" --quiet -- verify -f pk -p 0 -s sig1
Fail
$ echo -n "msg" | cargo run --features="kes_cli" --quiet -- verify -f pk -p 1 -s sig1
OK
```

## Compatibility with Cardano
We provide two implementations of KES for compatibility with Cardano's blockchain. Cardano currently
uses `Sum6Kes` which is suppoted here.
As that implementation is not optimal in what concerns signature size,
we provide implementation of `SumCompact6Kes`, which provides an asymptotic halving of the signature
size. We provide test vectors generated using Cardano's code to ensure that future changes in the
library will not lose compatibility with Cardano. These test vectors can be found in `./data`,
and the tests can be found in `summed_kes_interoperability.rs`.

## Interoperability with cardano-node and cardano-cli

Secret keys of this crate are not compatible with KES keys as they are used in the
[cardano node](https://github.com/IntersectMBO/cardano-node). In this crate we include the
period of the KES secret key as part of its structure, while the cardano implementation does not.
This decision is motivated by two reasons:
* It considerably simplifies the API and makes it more intuitive to use. Moreover, the period is
  a required knowledge to sign/update a skey, and we concluded that a secret key should contain it's
  period.
* Secret keys are not send through the wire, meaning that a node using this implementation will not
  need to be compatible with cardano node's serialisation. However, if for some reason one needs to
  serialise a cardano node serialised key for usage in this application (or vice-versa), one simply
  needs to add the period as a 32 bit number represented in 4 big endian bytes (or, vice-versa,
  remove the last 4 bytes from the serialised signature). An example of such a procedure can be found
  in the [interoperability](summed_kes_interoperability.rs) tests of this crate.

Taking all the remarks above here is how we can seamlessly use the command line to use with
[cardano cli](https://github.com/IntersectMBO/cardano-cli)

```console
$ cardano-cli --version
cardano-cli 10.11.1.0 - linux-x86_64 - ghc-9.6

# Creating example KES keys, two files are created : kes.skey and kes.vkey
# We also have crfa.kes.counter which stores current period
$ cardano-cli conway node key-gen-KES --verification-key-file kes.vkey --signing-key-file kes.skey --key-output-bech32
$ cat kes.skey
kes_sk1emm33cuyf0hczrf4wxzhcfc2maa7ujmelu4g6kmjecmsx8tg3sw8jnhcmqj0vfvv37g2nhamalmaa8zyue4jl49hjupwzy08jpftjhpu3w2zdc823z6ns768l7xltxjd5cv6w5z34r6hmp6dtupq4nk6re6pxly93v6q5v0adpk5lelv0hyrr8wzn6u56an5qqqeh7nctav56r3gpvr4m9cc4f80g79jvvuclgvw3ltwst0nrf98p925wusl0a0cpc557uuhnv0k3dyrdd6fkgdwfymgraq0ezmlyjned2j9qfhpqljjzcmhzlpcazwknku3sae46kek7zf3gmnd7qyusd8c4yu2xtja537k7t0k4m9zc56ds6lsp6lfm7ggqwvec77gudqlp7lmx8j7tdxz0vdj9gjn59m88hy23pzsslldkmv6d4t96vksjr3krggzy3pvw69gs8rl2ef4mgjuv7njwsy8dpsjxn6e0dncfzffqdthch03ruktw4xn65ql3rarlyhdj6w5ulxtq9mrdvcy9rhqkwumyqfuzcpnz2gmr3fnd0yqcluycnrfx47r7tfps0ulkl0tgruf9rugtmy8x7crhterw0ry33g2qxxarr6q6vnenl9tx6wr7cfy0fe2h5xsqcn579ql29c337ndaj2xff398707wj7st3nat3ftl6e3p3ypjzpldrlwzfpyta27gwvvv80r7pe8rn7pmvly8h0dlgy7t4a0u4lt7hxa2kpeuet0hgdyest73wj57sflwxrnmsgd36pfelphqhftz5tnr6klhdj6qtffyhhqe0h3l2c646hlgawdxwhhy03xfsj7trt7sms7qfy2xd4a0kqetfrjxa7x3vxnrsx4sftgm3yjwj0tmtmgu5846w8fc0x07ya5w826mr2jphe7efqykpmaal4xemhf82s35wzhql3ygja8npjtxaa2nyykga7zl4y
$ cat kes.vkey
kes_vk1ks9vm6c736u4xx6g25zfvcw5xzhewtvdctl858qn3zemzhp03n2s8empqg
$ cat crfa.kes.counter
000

# Now having kes.skey and period (stored here as decimal) let's create skey (hex-decoded key with appended 32-byte period)
$ echo "0" | printf '%08X\n' $(</dev/stdin)
00000000
$ cat kes.skey | bech32 | echo -n $(</dev/stdin) > skey
$ echo "0" | printf '%08X\n' $(</dev/stdin) >> skey
$ cat skey
cef718e3844bef810d3571857c270adf7bee4b79ff2a8d5b72ce37031d688c1c794ef8d824f6258c8f90a9dfbbeff7de9c44e66b2fd4b79702e111e79052b95c3c8b9426e0ea88b5387b47ff8df59a4da619a75051a8f57d874d5f020aceda1e74137c858b340a31fd686d4fe7ec7dc8319dc29eb94d767400019bfa785f594d0e280b075d9718aa4ef478b263398fa18e8fd6e82df31a4a7095547721f7f5f80e294f73979b1f68b4836b749b21ae493681f40fc8b7f24a796aa45026e107e521637717c38e89d69db9187735d5b36f093146e6df009c834f8a938a32e5da47d6f2df6aeca2c534d86bf00ebe9df90803999c7bc8e341f0fbfb31e5e5b4c27b1b22a253a17673dc8a8845087fedb6d9a6d565d32d090e361a1022442c768a881c7f56535da25c67a72740876861234f597b6784892903577c5df11f2cb754d3d501f88fa3f92ed969d4e7ccb017636b30428ee0b3b9b2013c160331291b1c5336bc80c7f84c4c69357c3f2d2183f9fb7deb40f8928f885ec8737b03baf2373c648c50a018dd18f40d32799fcab369c3f61247a72abd0d006274f141f517118fa6dec9464a6253f9fe74bd05c67d5c52bfeb310c4819083f68fee124245f55e4398c61de3f07271cfc1db3e43ddedfa09e5d7afe57ebf5cdd55839e656fba1a4cc17e8ba54f413f71873dc10d8e829cfc3705d2b151731eadfbb65a02d2925ee0cbef1fab1aaeaff475cd33af723e264c25e58d7e86e1e0248a336bd7d8195a472377c68b0d31c0d582568dc492749ebdaf68e50f5d38e9c3ccff13b471d5ad8d520df3eca404b077defea6ceee93aa11a385707e2444ba79864b377aa99096400000000
$ Having that we can recreate kes.vkey
$ cargo run --features="kes_cli" --quiet -- --derive_pk skey | bech32 kes_vk
kes_vk1ks9vm6c736u4xx6g25zfvcw5xzhewtvdctl858qn3zemzhp03n2s8empqg

# Updating the key
$ cargo run --features="kes_cli" --quiet -- --update_sk skey > skey1
$ cat skey1
794ef8d824f6258c8f90a9dfbbeff7de9c44e66b2fd4b79702e111e79052b95c00000000000000000000000000000000000000000000000000000000000000003c8b9426e0ea88b5387b47ff8df59a4da619a75051a8f57d874d5f020aceda1e74137c858b340a31fd686d4fe7ec7dc8319dc29eb94d767400019bfa785f594d0e280b075d9718aa4ef478b263398fa18e8fd6e82df31a4a7095547721f7f5f80e294f73979b1f68b4836b749b21ae493681f40fc8b7f24a796aa45026e107e521637717c38e89d69db9187735d5b36f093146e6df009c834f8a938a32e5da47d6f2df6aeca2c534d86bf00ebe9df90803999c7bc8e341f0fbfb31e5e5b4c27b1b22a253a17673dc8a8845087fedb6d9a6d565d32d090e361a1022442c768a881c7f56535da25c67a72740876861234f597b6784892903577c5df11f2cb754d3d501f88fa3f92ed969d4e7ccb017636b30428ee0b3b9b2013c160331291b1c5336bc80c7f84c4c69357c3f2d2183f9fb7deb40f8928f885ec8737b03baf2373c648c50a018dd18f40d32799fcab369c3f61247a72abd0d006274f141f517118fa6dec9464a6253f9fe74bd05c67d5c52bfeb310c4819083f68fee124245f55e4398c61de3f07271cfc1db3e43ddedfa09e5d7afe57ebf5cdd55839e656fba1a4cc17e8ba54f413f71873dc10d8e829cfc3705d2b151731eadfbb65a02d2925ee0cbef1fab1aaeaff475cd33af723e264c25e58d7e86e1e0248a336bd7d8195a472377c68b0d31c0d582568dc492749ebdaf68e50f5d38e9c3ccff13b471d5ad8d520df3eca404b077defea6ceee93aa11a385707e2444ba79864b377aa99096400000001
# kes.skey is skey1 without the last 4 bytes, i.e.,
$ cat skey1 | head -c -8 | bech32 kes_sk > kes.skey1
$ cat kes.skey1
kes_sk109803kpy7cjcerus480mhmlhm6wyfent9l2t09czuyg70yzjh9wqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpu3w2zdc823z6ns768l7xltxjd5cv6w5z34r6hmp6dtupq4nk6re6pxly93v6q5v0adpk5lelv0hyrr8wzn6u56an5qqqeh7nctav56r3gpvr4m9cc4f80g79jvvuclgvw3ltwst0nrf98p925wusl0a0cpc557uuhnv0k3dyrdd6fkgdwfymgraq0ezmlyjned2j9qfhpqljjzcmhzlpcazwknku3sae46kek7zf3gmnd7qyusd8c4yu2xtja537k7t0k4m9zc56ds6lsp6lfm7ggqwvec77gudqlp7lmx8j7tdxz0vdj9gjn59m88hy23pzsslldkmv6d4t96vksjr3krggzy3pvw69gs8rl2ef4mgjuv7njwsy8dpsjxn6e0dncfzffqdthch03ruktw4xn65ql3rarlyhdj6w5ulxtq9mrdvcy9rhqkwumyqfuzcpnz2gmr3fnd0yqcluycnrfx47r7tfps0ulkl0tgruf9rugtmy8x7crhterw0ry33g2qxxarr6q6vnenl9tx6wr7cfy0fe2h5xsqcn579ql29c337ndaj2xff398707wj7st3nat3ftl6e3p3ypjzpldrlwzfpyta27gwvvv80r7pe8rn7pmvly8h0dlgy7t4a0u4lt7hxa2kpeuet0hgdyest73wj57sflwxrnmsgd36pfelphqhftz5tnr6klhdj6qtffyhhqe0h3l2c646hlgawdxwhhy03xfsj7trt7sms7qfy2xd4a0kqetfrjxa7x3vxnrsx4sftgm3yjwj0tmtmgu5846w8fc0x07ya5w826mr2jphe7efqykpmaal4xemhf82s35wzhql3ygja8npjtxaa2nyykgygustn
# period counter now is incremented
$ cat crfa.kes.counter
001
# kes verifiation key stays the same
$ cargo run --features="kes_cli" --quiet -- --derive_pk skey1 | bech32 kes_vk
kes_vk1ks9vm6c736u4xx6g25zfvcw5xzhewtvdctl858qn3zemzhp03n2s8empqg

# Please take notice that skeyN takes hex-decoded period and crfa.kes.counter is decimal number.
$ echo "20" | printf '%08X\n' $(</dev/stdin)
00000014
```

## Previous versions of the code
This repo is an adapted copy of
[txpipe/kes](https://github.com/txpipe/kes.git), which in turn is fork of
[kes-mmm-sumed25519](https://github.com/IntersectMBO/kes-mmm-sumed25519). The old repo
remains unchanged for historical purposes.

## Disclaimer
This crate has not been audited. Use at your own risk.

## Contribution
Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, shall be licensed
as above, without any additional terms or conditions.
