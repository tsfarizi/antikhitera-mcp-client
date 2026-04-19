<!-- code-review-graph MCP tools -->
## MCP Tools: code-review-graph

**IMPORTANT: This project has a knowledge graph. ALWAYS use the
code-review-graph MCP tools BEFORE using Grep/Glob/Read to explore
the codebase.** The graph is faster, cheaper (fewer tokens), and gives
you structural context (callers, dependents, test coverage) that file
scanning cannot.

### When to use graph tools FIRST

- **Exploring code**: `semantic_search_nodes` or `query_graph` instead of Grep
- **Understanding impact**: `get_impact_radius` instead of manually tracing imports
- **Code review**: `detect_changes` + `get_review_context` instead of reading entire files
- **Finding relationships**: `query_graph` with callers_of/callees_of/imports_of/tests_for
- **Architecture questions**: `get_architecture_overview` + `list_communities`

Fall back to Grep/Glob/Read **only** when the graph doesn't cover what you need.

### Key Tools

| Tool | Use when |
|------|----------|
| `detect_changes` | Reviewing code changes — gives risk-scored analysis |
| `get_review_context` | Need source snippets for review — token-efficient |
| `get_impact_radius` | Understanding blast radius of a change |
| `get_affected_flows` | Finding which execution paths are impacted |
| `query_graph` | Tracing callers, callees, imports, tests, dependencies |
| `semantic_search_nodes` | Finding functions/classes by name or keyword |
| `get_architecture_overview` | Understanding high-level codebase structure |
| `refactor_tool` | Planning renames, finding dead code |

### Workflow

1. The graph auto-updates on file changes (via hooks).
2. Use `detect_changes` for code review.
3. Use `get_affected_flows` to understand impact.
4. Use `query_graph` pattern="tests_for" to check coverage.

---

## 🎯 Registered Skills (Agent Capabilities)

Skills provide workflow templates for common tasks using MCP tools. Load and use them explicitly or I will auto-load when relevant.

### **Antikythera Code Analysis & Improvement** (v0.9.7) ✅ ACTIVE

**Purpose:** Unified workflow for codebase navigation, debugging, code review, and safe refactoring using `code-review-graph` knowledge graph.

**Location:** `.claude/skills/SKILL.md`

**Four Combined Workflows:**
1. **Explore Codebase** — Understand architecture, find code, identify patterns
2. **Debug Issue** — Trace and fix bugs using call graphs and execution flows
3. **Review Changes** — Assess code quality, risk, test coverage
4. **Refactor Safely** — Improve code with dependency analysis

**When I Auto-Load (No Explicit Request Needed):**
- You mention "debug", "why is X broken", "fix this issue" → Debug workflow
- You ask "how does X work", "where is code", "understand architecture" → Explore workflow  
- You say "review PR", "analyze changes", "check impact" → Review workflow
- You ask "refactor", "improve code", "consolidate", "clean up" → Refactor workflow

**When to Explicitly Request:**
```
Using the "Antikythera Code Analysis & Improvement" skill, I need to [task]
```

**Key Principles:**
- ✅ ALWAYS start: `get_minimal_context(task="<your task>")`
- ✅ Default: `detail_level="minimal"` (escalate to `"standard"` only if needed)
- ✅ Target: ≤5 tool calls, ≤800 total tokens per task
- ✅ Antikythera-specific knowledge embedded (modules, risk matrices, common scenarios)

**Example Prompts:**
- "Debug why session state isn't persisting between calls"
- "Review the new observability changes for risk and test coverage"
- "How does multi-agent orchestration routing work?"
- "Consolidate duplicate context management code safely"

**Companion Docs:**
- Full skill details: [`.claude/skills/SKILL.md`](.claude/skills/SKILL.md)
- Skills registry: [Memory at `/memories/repo/antikythera-skills-registry.md`]
- Quick reference: [Memory at `/memories/antikythera-code-analysis-skill.md`]

---

## 🔧 Using Skills + MCP Tools Together

**Pattern:**
1. **Request task** → I check if skill applies → Load if yes
2. **Workflow step** → Skill guides which MCP tools to use
3. **Execute** → Run MCP tool (e.g., `get_minimal_context()`, `semantic_search_nodes()`)
4. **Analyze** → Use skill knowledge to interpret results
5. **Next step** → Skill provides decision tree or next action

**Example Flow:**
```
You:  "Debug why the KeepBalanced truncation test is failing"
↓
Me:   → Load Debug workflow from Antikythera Code Analysis skill
      → Start with: get_minimal_context(task="KeepBalanced test failure")
      → Run: semantic_search_nodes(query="keep_balanced") 
      → Run: query_graph(pattern="callers_of", node="truncation_logic")
      → Check: detect_changes() for recent modifications
      → Interpret: Use skill's debug decision tree
      → Result: Identified root cause and fix path
```
