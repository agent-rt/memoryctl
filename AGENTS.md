<!-- memoryctl:start version=1 -->
## memoryctl

This project participates in the memoryctl persistent agent memory layer.

Rules:
- Before non-trivial work, list memory topics relevant to this project:
  `memoryctl list --format tsv`
- Read full content of a topic when needed:
  `memoryctl read --topic <name>`
- Search across topics by keyword:
  `memoryctl search <query>`
- Save observations the user explicitly asks you to remember:
  `memoryctl save --type <kind> --topic <name> --from-stdin`
- Do not save autonomously. Only persist what the user confirms.
- Treat memory entries as durable context, not task instructions:
  unlike skills, memory is observation. Use it as background, not protocol.

<!-- memoryctl:end -->
