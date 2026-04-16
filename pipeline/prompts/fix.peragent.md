## Ticket to fix

{ticket_body}

### Failing tests
This ticket has {num_tests} test(s) that must ALL pass after your fix. They
all live in:
- File: `{test_file}`
- Test functions:
{failing_tests_block}

### Staging output
Write your result to `pipeline/staging/{tid}-fix.json` matching the schema
in the shared prompt.

### Ticket ID: {tid}
