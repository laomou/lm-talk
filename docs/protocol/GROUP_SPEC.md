# 群组规格 v1

群组对象包括群邀请、群事件、群 fanout 和 Group Sender Key。

## 文本对象

```text
lm-group-invite-v1:<base64url-json>
lm-group-event-v1:<base64url-json>
lm-group-sender-key-v1:<base64url-json>
```

## 投递模型

群聊采用成员 fanout：

- 没有 Sender Key 时，对每个成员分别加密 direct envelope。
- 使用 Sender Key 时，生成 group sender envelope，再 fanout 给成员。
- 群事件作为带群事件前缀的 direct 加密载荷投递给成员。

## 成员和历史

- 新成员不会自动获得历史消息。
- 历史转移必须由现有成员明确触发，并重新加密。
- 群事件包含 sequence，用于防止乱序和重复应用。

## strict E2EE

创建群、接受群邀请、发送群消息和群事件 fanout 都会检查群成员风险。strict 模式下会阻止核心风险路径。

测试向量：

- `test-vectors/group_v1.json`
- `test-vectors/group_sender_key_v1.json`
