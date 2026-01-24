//! Tests for left recursion detection and prevention.
//!
//! Left recursion causes infinite loops in trampoline-based parsers.
//! Example: A rule that calls itself (directly or indirectly) at the start.
//!
//! BAD (left recursion):
//!   array_type -> primary_type + "[]"
//!   primary_type -> array_type | other_type
//!
//! GOOD (no left recursion, use postfix):
//!   primary_type -> base_type + "[]"*
//!   base_type -> other_type

// Note: We don't have a test grammar for left recursion because
// it would cause the test to hang infinitely. The fix is documented here:
//
// If you have left recursion, restructure the grammar to use postfix:
//   Instead of: rule -> rule + suffix
//   Use: rule -> base + suffix*
//
// This was discovered when array types in TypeScript caused infinite loops:
//   array_type -> primary_type + []
//   primary_type -> ... | array_type | ...
//
// Fixed by making array types postfix:
//   primary_type -> base_type + []*
//   base_type -> ... (no array_type)
