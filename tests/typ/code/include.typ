// Test including file contents with import.

---
= Document

// Include a file
#import "importable/chap1.typ"

// The variables of the file should not appear in this scope.
// Error: 1-6 unknown variable
#name

// Expression as a file name.

_ -- Intermission -- _
#import "import" + "able/chap" + "2.typ"

{
    // Expressions, code mode.
    // Error: 12-34 file not found
    import "importable/chap3.typ"
}
