# LM Talk Public Peer / Mailbox 规格 v1

Public peer announce 会声明 peer id、地址、能力、配额和过期时间。Mailbox 消息由发送方身份签名，并包含接收方 UserID、kind、密文载荷、创建时间和过期时间。

Mailbox kind 包括 SignalOffer、SignalAnswer、DirectEnvelope、GroupFanout 和 Other。节点会验签并保存投递，直到客户端 take 并 ack。

生产级反滥用要求：配额、TTL、最大消息大小、限流或 proof-of-work，以及重复消息抑制。
