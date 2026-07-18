# LM Talk 群组规格 v1

群组文本格式示例：

```text
lm-group-event-v1:<base64url-json>
lm-group-invite-v1:<base64url-json>
```

MVP 群组使用成员 fanout：

- 未使用 Sender Key 时：为每个成员分别加密一个 direct envelope。
- 使用 Sender Key 时：加密 group sender envelope，并将该 envelope fanout 给成员。
- 群事件是带签名的协议对象，作为带群事件前缀的 direct 加密载荷投递。

新成员不会自动收到历史消息。历史转移必须由现有成员明确触发，并重新加密。

测试向量见 `test-vectors/group_v1.json` 和 `test-vectors/group_sender_key_v1.json`。
