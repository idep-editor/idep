# Idep — Product Definition

> *Companion to [README.md](README.md). Source of truth for "what is this, and who is it for".*

---

## Statement

Idep is a native, Rust-built AI IDE for environments where mainstream IDEs
fail three specific constraints at once:

1. **Hardware is constrained.** 4–8 GB RAM laptops. No dedicated GPU.
2. **Networks are unreliable.** Intermittent, slow, or metered connections.
3. **Code must stay on the machine.** Sovereign, regulated, or air-gapped deployments.

Every other "lightweight AI editor" fails at least one of these. VS Code forks
fail on (1). Cloud-first editors fail on (2) and (3). Generic native editors
(Helix, Zed) don't prioritize (1) because their users rarely need it.

Idep is the editor for developers who need all three at once.

---

## Target user

The primary target user is a working developer who:

- Uses a 4–16 GB RAM machine as a daily driver.
- Works over WSL2, SSH, or both.
- Relies on Ollama or an equivalent local model server as a default, not fallback.
- Operates under one or more of: bandwidth constraints, code residency rules,
  enterprise air-gap policy, or personal preference for local-first tooling.
- Reads docs in English and is comfortable configuring tools via TOML.

Regional context: Idep is built in Bali and initial community outreach
(see [TODO.md](TODO.md) v0.8.4) targets ASEAN and Indonesian developer
communities, where the three constraints above are common by default rather
than exception. This is a starting audience, not a ceiling.

**Secondary users** (welcome, but not the North Star):

- Privacy-conscious developers on better-equipped hardware.
- Teams evaluating sovereign-deployment dev tooling.
- Contributors interested in Rust-native editor architecture.

---

## Non-goals

Idep will not:

- Ship a proprietary cloud service, paid tier, or enterprise feature gate.
- Build a VS Code extension marketplace or attempt extension-API parity.
- Compete with Cursor/Windsurf on agent-orchestration breadth.
- Compete with Zed on GPU-rendered typography polish.
- Bundle a specific AI model. Model choice stays with the user.
- Add telemetry, analytics, update checks, or any unsolicited network call.
- Freeze the plugin API before real community plugins stress-test it.
- Add features that materially raise the editor's idle RAM floor.

These are deliberate constraints. They make the product smaller, sharper,
and easier to trust. Every "also does X" request will be measured against
this list first.

---

## What success looks like

**Technical (by v1.0.0):**

- `<2 GB` editor RSS when actively editing a 10k-line file.
- `<500 MB` editor RSS idle.
- Zero unsolicited network calls verified by `tests/network_audit.rs`.
- First AI completion in `<15 min` from fresh install with Ollama pre-cached.
- 50k LOC project indexed on a 4-core machine within the v0.5.1 target band.
- 100% of `SECURITY.md` claims backed by a named test.

**Community (by v1.0.0):**

- At least one external contributor with a merged PR.
- At least one working community-authored plugin.
- Indonesian-language getting-started guide live.
- Measured benchmarks on 4 GB RAM hardware documented.

**Strategic:**

- Zed will not ship the three-constraint solution above. If a reader asks
  "why not just use Zed?", the answer is a single sentence pointing to one
  of the three constraints.
- Cursor, Windsurf, Antigravity will not ship an Apache 2.0, offline-capable,
  in-process-RAG alternative. Their business models prevent it.

That's the moat.

---

## What Idep is *not*

- Not a Vim replacement. Vim is better at being Vim.
- Not a Neovim replacement. Neovim has a richer plugin ecosystem and will continue to.
- Not a Helix replacement. Helix is stricter about plugin minimalism and
  faster to install. If Helix's constraints fit, use Helix.
- Not a Zed competitor on features. Zed will ship more editor polish faster.
- Not a Cursor competitor on agent breadth. Cursor will ship more agent
  features faster.

Idep competes on the intersection of Apache-2.0 license + native runtime +
in-process RAG + constrained-hardware target + first-class offline backend.
That intersection is currently empty. Staying in it is the work.

---

## How decisions get made

When a feature request or architectural choice arrives, it passes through
these questions in order:

1. Does this raise idle RAM above the `<500 MB` target? → Reject or defer.
2. Does this require a network call the user didn't explicitly trigger? → Reject.
3. Does this compromise the plugin sandbox? → Reject.
4. Does this add a dependency with a single maintainer or incompatible license? → Reject
   or document mitigation.
5. Would Zed or Cursor ship this for free within six months? → Deprioritize
   unless it serves the three-constraint target user specifically.
6. Does a community contributor want to own it? → Accept, with scope.

These are not aspirational. They are the filter.

---

*Version: aligned with Idep v0.1.0, 2026-04-21.*  
*Changes to this document must be announced in `CHANGELOG.md`.*
