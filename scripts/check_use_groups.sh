#!/bin/bash
# Detects blank lines between consecutive `use` statements in .rs files.
# Usage: ./scripts/check_use_groups.sh [directory]
# Exit code: 0 if clean, 1 if violations found.

set -euo pipefail
dir="${1:-.}"

violations=0

while IFS= read -r file; do
    # Use perl to find consecutive use lines separated by blank lines
    # Pattern: a `use ` line, then a blank line, then another `use ` line
    matches=$(perl -ne '
        BEGIN { $prev_was_use = 0; $line_num = 0; }
        $line_num++;
        if (/^\s*use\s/) {
            if ($prev_was_use == -1) {
                print "$ARGV:$prev_line_num: blank line before use at line $line_num\n";
            }
            $prev_was_use = 1;
            $prev_line_num = $line_num;
        } elsif (/^\s*$/) {
            if ($prev_was_use == 1) {
                $prev_was_use = -1;  # blank line after use
            } else {
                $prev_was_use = 0;
            }
        } else {
            $prev_was_use = 0;
        }
    ' "$file")

    if [ -n "$matches" ]; then
        echo "$matches"
        violations=$((violations + 1))
    fi
done < <(find "$dir" -name '*.rs' -not -path '*/target/*')

if [ "$violations" -gt 0 ]; then
    echo ""
    echo "Found blank lines between use groups. Per AGENTS.md:"
    echo "  Do not separate use groups with a blank line."
    echo "  All use statements should be in a single block."
    exit 1
fi

exit 0