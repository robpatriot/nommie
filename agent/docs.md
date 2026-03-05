# Documentation Rules

These rules apply to all documentation in `docs/`.

## Purpose
Documents are specifications that explain how a topic works in this codebase.  
They must be concise, technically focused, and useful to both developers and agents making changes.

## Orthogonality
Each document owns a single topic.  
Documents must not significantly duplicate or overlap.  

## Accuracy
Documentation must describe the current behavior of the system, not future plans.  
If code changes invalidate documentation, the documentation must be updated in the same change.

## Concision
Prefer short, dense documents that can be read quickly.  
Avoid narrative explanations, historical notes, or roadmap content.

## Code examples
Avoid code listings, include code only when it materially improves understanding of an interface, invariant, or data structure.

## Referencing
Prefer linking to other documents rather than repeating their content.
