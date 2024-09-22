# (2024-09-22) Version 1.1.1

- (**Bugfix**) Fix thread messages being lost due to lack of pagination support ([d4ddd29660](https://github.com/etkecc/baibot/commit/d4ddd29660d9f51d248119dd6032e68ab29e7d35)) - fixes [issue #13](https://github.com/etkecc/baibot/issues/13)

- (**Bugfix**) Fix Anthropic conversations getting stuck when being impatient and sending multiple consecutive messages ([8b12bdf2b3](https://github.com/etkecc/baibot/commit/8b12bdf2b3196abea0e8db33d7c50fff48341cb9)) - fixes [issue #13](https://github.com/etkecc/baibot/issues/13)


# (2024-09-21) Version 1.1.0

- (**Feature**) Adds support for [prompt variables](./docs/configuration/text-generation.md#️-prompt-override) (date/time, bot name, model id) ([2a5a2d6a4d](https://github.com/etkecc/baibot/commit/2a5a2d6a4dbf5fd7cb504ac07d4187fdc32ae395)) - fixes [issue #10](https://github.com/etkecc/baibot/issues/10)

- (**Improvement**) [Dockerfile](./Dockerfile) changes to produce ~20MB smaller container images ([354063abb7](https://github.com/etkecc/baibot/commit/354063abb79035069bd3b26c53214874e9cdd95d))

- (**Improvement**) [Dockerfile](./Dockerfile) changes to optimize local (debug) runs in a container ([c8c5e0e540](https://github.com/etkecc/baibot/commit/c8c5e0e540ab981e849452eb3ddb0378105e1fc6))

- (**Improvement**) CI changes to try and work around multi-arch image issues like [this one](https://github.com/etkecc/baibot/issues/2) ([5de7559ed6](https://github.com/etkecc/baibot/commit/5de7559ed685a41c22dfc12283681f02f4c2ee00))


# (2024-09-19) Version 1.0.6

Improvements to:

- messages sent by the bot - better onboarding flow, especially when no agents have been created yet
- documentation pages


# (2024-09-14) Version 1.0.5

Further [improves](https://github.com/etkecc/baibot/commit/3b25b92a81a05ebaf1c6dbabf675fbfbe6c9f418) the typing notification logic, so that it tolerates edge cases better.


# (2024-09-14) Version 1.0.4

[Improves](https://github.com/etkecc/baibot/commit/dd1dd78312e3db7f92b37fb3b4750fbe35de7115) the typing notification logic.


# (2024-09-13) Version 1.0.3

Contains [fixes](https://github.com/etkecc/rust-mxlink/commit/f339fc85e69aa7f614394ad303d1614cd307319c) for [some](https://github.com/etkecc/baibot/issues/1) startup failures caused by partial initialization (errors during startup).


# (2024-09-12) Version 1.0.0

Initial release. 🎉
