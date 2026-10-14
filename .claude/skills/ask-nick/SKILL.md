---
name: ask-nick
description: Ask Nick (the owner) a product decision about Shuyi Engine and record his answer in docs/decisions.md — the name, licenses, going public, contributions, what is parked or unparked, what the game gets, anything costing money or legal duties. Never use for technical questions.
---

# Ask Nick a product decision

Nick decides the **product**. Claude decides **everything technical** and
never asks Nick about languages, libraries, architecture or code. Nick
likes questions with options drawn from real projects, a
recommendation, and room to describe his own.

## What counts as his

- The engine's name and branding.
- Licenses, and anything touching the game's `LICENSE`.
- What goes public, and when.
- Who may contribute.
- Which parked tickets start.
- Anything that changes what the game gets or when.
- Anything that costs money or brings legal duties: servers, accounts,
  chat, a trademark.

## Format for each question

Ask at most **3 questions per message**. Use `AskUserQuestion` when it's
available; the buttons and the "Other" answer are how he replies best.
For each question:

```
### <Question in plain words>

**A. <Name> — like <real project>**
What it means here: <1–2 sentences, concrete>.

**B. …** (2–4 options, genuinely different)

**My recommendation:** <letter> — <one sentence tied to what Nick already decided>.

**Or describe your own.**
```

Rules:

- Say plainly what each option costs: time, money, the game's velocity.
- Name facts you checked and where (a crate's license, a price page,
  dated). Flag what you couldn't check.
- Nothing legal is advice. Say when a lawyer's look is worth it.
- If he answers vaguely, take the recommendation, say exactly what you're
  recording, and let him veto.

## Recording the answer

1. Add a dated part to `docs/decisions.md` with his words **verbatim**,
   then the rule that follows from them.
2. Update the tickets the answer changes (unpark, re-scope, drop) in the
   same PR, and `docs/ROADMAP.md` if phases change.
3. If the answer changes the game, write or update the game repository's
   ticket and link it.
