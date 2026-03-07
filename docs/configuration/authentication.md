## 🔐 Authentication

baibot supports 2 authentication modes for the Matrix account (`user.*` keys in config).

Set **exactly one** mode. If both are set (or neither is set), startup validation fails.

### Password authentication

- Config key: `user.password`
- Environment variable: `BAIBOT_USER_PASSWORD`

### Access token authentication

- Config keys: `user.access_token` + `user.device_id`
- Environment variables: `BAIBOT_USER_ACCESS_TOKEN` + `BAIBOT_USER_DEVICE_ID`

Access-token authentication is useful for OIDC-enabled homeservers (e.g. those using [Matrix Authentication Service](https://github.com/element-hq/matrix-authentication-service)).

Example token-generation command:

```sh
mas-cli manage issue-compatibility-token <username> [device_id]
```
