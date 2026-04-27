<!-- memctl:start version=1 -->
## memctl

This project participates in the memctl persistent agent memory layer.

Rules:
- Before non-trivial work, list memory topics relevant to this project:
  `memctl list --format tsv`
- Read full content of a topic when needed:
  `memctl read --topic <name>`
- Search across topics by keyword:
  `memctl search <query>`
- Save observations the user explicitly asks you to remember:
  `memctl save --type <kind> --topic <name> --from-stdin`
- Do not save autonomously. Only persist what the user confirms.
- Treat memory entries as durable context, not task instructions:
  unlike skills, memory is observation. Use it as background, not protocol.

<!-- memctl:end -->
