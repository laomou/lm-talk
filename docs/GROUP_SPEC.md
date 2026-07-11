# LM Talk Group Spec v1

MVP groups use member fanout:

- Without Sender Key: encrypt one direct envelope per member.
- With Sender Key: encrypt group sender envelope and fan out the envelope to members.
- Group events are signed protocol objects and are delivered as direct encrypted payloads with a group event prefix.

New members do not receive history automatically. History transfer must be explicit and re-encrypted by an existing member.
