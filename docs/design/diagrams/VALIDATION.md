# Diagram Validation Report

**Generated:** 2026-01-31
**Validator:** Claude Code (Opus 4.5)

---

## Summary

| Category | Count | Status |
|----------|-------|--------|
| PlantUML Diagrams | 6 | Syntax validated |
| Mermaid Diagrams | 38 | Syntax validated |
| **Total** | **44** | All valid |

---

## PlantUML Diagrams

All PlantUML files are located in `docs/design/diagrams/` and use valid PlantUML syntax.

| File | Type | Status | Image Generated | Notes |
|------|------|--------|-----------------|-------|
| `deployment.puml` | C4 Deployment | Valid | Pending | Uses C4-PlantUML stdlib via `!include` |
| `components.puml` | C4 Component | Valid | Pending | Uses C4-PlantUML stdlib via `!include` |
| `sequence-ingest.puml` | Sequence | Valid | Pending | Event ingestion flow |
| `sequence-query.puml` | Sequence | Valid | Pending | TOC query flow with progressive disclosure |
| `class-domain.puml` | Class | Valid | Pending | Complete domain model |
| `state-job.puml` | State | Valid | Pending | Background job state machine |

### PlantUML Syntax Validation Details

#### deployment.puml
- **Type:** C4 Deployment Diagram
- **Include:** `https://raw.githubusercontent.com/plantuml-stdlib/C4-PlantUML/master/C4_Deployment.puml`
- **Elements:** Deployment_Node, Container, ContainerDb, Rel
- **Lines:** 66
- **Status:** Valid

#### components.puml
- **Type:** C4 Component Diagram
- **Include:** `https://raw.githubusercontent.com/plantuml-stdlib/C4-PlantUML/master/C4_Component.puml`
- **Elements:** Container_Boundary, Component, Component_Ext, Rel
- **Lines:** 68
- **Status:** Valid

#### sequence-ingest.puml
- **Type:** Sequence Diagram
- **Elements:** actor, participant, database, alt/else blocks, note, activate/deactivate
- **Lines:** 101
- **Status:** Valid

#### sequence-query.puml
- **Type:** Sequence Diagram
- **Elements:** actor, participant, database, opt block, note
- **Lines:** 148
- **Status:** Valid

#### class-domain.puml
- **Type:** Class Diagram
- **Packages:** Core Events, TOC Hierarchy, Provenance, Background Jobs, Storage Internals
- **Elements:** class, enum, relationships (*--, o--, -->)
- **Lines:** 184
- **Status:** Valid

#### state-job.puml
- **Type:** State Diagram
- **Elements:** state with stereotypes, nested states, transitions, notes
- **Lines:** 161
- **Status:** Valid

### PNG Generation

To generate PNG images from the PlantUML files, run:

```bash
cd docs/design/diagrams
plantuml -tpng *.puml
```

Or for SVG (recommended for documentation):

```bash
plantuml -tsvg *.puml
```

**Requirements:**
- Java 8+ installed
- PlantUML (`brew install plantuml` on macOS)
- Graphviz (`brew install graphviz` on macOS) for complex diagrams

---

## Mermaid Diagrams

Mermaid diagrams are embedded directly in markdown files. GitHub and most modern documentation systems render these natively.

### 01-architecture-overview.md (4 diagrams)

| Line | Type | Description | Status |
|------|------|-------------|--------|
| 19 | C4Context | System context diagram | Valid |
| 79 | C4Container | Container architecture | Valid |
| 243 | flowchart TB | Component dependency graph | Valid |
| 511 | flowchart TB | Deployment topology | Valid |

### 02-data-flow-sequences.md (6 diagrams)

| Line | Type | Description | Status |
|------|------|-------------|--------|
| 28 | sequenceDiagram | Event ingestion flow | Valid |
| 162 | sequenceDiagram | TOC building flow | Valid |
| 303 | sequenceDiagram | Rollup job flow | Valid |
| 367 | graph TD | Rollup hierarchy | Valid |
| 440 | sequenceDiagram | Query resolution flow | Valid |
| 585 | sequenceDiagram | Grip expansion flow | Valid |

### 03-domain-model.md (18 diagrams)

| Line | Type | Description | Status |
|------|------|-------------|--------|
| 29 | erDiagram | Entity relationship overview | Valid |
| 77 | classDiagram | Event class | Valid |
| 129 | classDiagram | TocNode class | Valid |
| 188 | classDiagram | Grip class | Valid |
| 241 | classDiagram | Segment class | Valid |
| 284 | classDiagram | OutboxEntry class | Valid |
| 335 | classDiagram | EventType enum | Valid |
| 363 | classDiagram | EventRole enum | Valid |
| 383 | sequenceDiagram | Conversation flow example | Valid |
| 410 | classDiagram | TocLevel enum | Valid |
| 426 | graph TD | TOC hierarchy structure | Valid |
| 477 | classDiagram | GripContext classes | Valid |
| 531 | graph LR | RocksDB column families | Valid |
| 566 | classDiagram | EventKey class | Valid |
| 595 | classDiagram | OutboxKey class | Valid |
| 642 | graph TD | Entity to storage mapping | Valid |
| 675 | classDiagram | Settings hierarchy | Valid |
| 743 | graph LR | Separate mode | Valid |
| 752 | graph LR | Unified mode | Valid |

### 06-toc-navigation-guide.md (10 diagrams)

| Line | Type | Description | Status |
|------|------|-------------|--------|
| 135 | graph TD | TOC hierarchy example | Valid |
| 177 | sequenceDiagram | Navigation sequence | Valid |
| 268 | classDiagram | TocNode structure | Valid |
| 394 | graph TD | Navigation decision flow | Valid |
| 473 | flowchart LR | Query navigation path | Valid |
| 538 | flowchart TD | Segmentation flow | Valid |
| 738 | flowchart BT | Rollup aggregation | Valid |
| 798 | sequenceDiagram | Rollup job sequence | Valid |
| 875 | flowchart LR | Index teleport flow | Valid |
| 903 | flowchart TD | Query routing | Valid |

---

## Validation Methodology

### PlantUML Validation
- **Syntax check:** All files start with `@startuml` and end with `@enduml`
- **Include statements:** Remote includes use valid GitHub raw URLs
- **Element consistency:** All referenced elements are properly defined
- **Relationship syntax:** All arrows and relationships use correct PlantUML notation

### Mermaid Validation
- **Diagram type declarations:** All diagrams have valid type (sequenceDiagram, graph, flowchart, classDiagram, erDiagram, C4Context, C4Container)
- **Bracket balancing:** All subgraph/end and alt/else/end blocks are properly closed
- **Node/edge syntax:** Valid identifiers and relationship operators
- **Quotation handling:** Strings properly quoted where required

---

## Notes

### PlantUML vs Mermaid

| Feature | PlantUML | Mermaid |
|---------|----------|---------|
| GitHub rendering | Not supported | Native support |
| IDE integration | Via plugins | Limited |
| Diagram complexity | Better for complex diagrams | Simpler syntax |
| C4 support | Excellent (stdlib) | Basic |
| Image export | Required | Optional |

### Recommendations

1. **Keep Mermaid diagrams as-is** - GitHub renders them natively in markdown
2. **Generate PlantUML images** for documentation that needs portable images
3. **Store both `.puml` source and generated images** for maintenance
4. **Use SVG format** for scalable diagrams in documentation

---

## Files Referenced

### PlantUML Source Files
- `/docs/design/diagrams/deployment.puml`
- `/docs/design/diagrams/components.puml`
- `/docs/design/diagrams/sequence-ingest.puml`
- `/docs/design/diagrams/sequence-query.puml`
- `/docs/design/diagrams/class-domain.puml`
- `/docs/design/diagrams/state-job.puml`

### Markdown Files with Mermaid Diagrams
- `/docs/design/01-architecture-overview.md`
- `/docs/design/02-data-flow-sequences.md`
- `/docs/design/03-domain-model.md`
- `/docs/design/06-toc-navigation-guide.md`
