Git Bug is an embedded bug tracker that uses git objects to store bug tracking data separately from the main file history. Key features include:

Listing Tickets:
```
# List all bugs
git bug bug

# List only open/closed bugs
git bug bug --status open/closed

# Sort by different criteria
git bug bug --by creation/edit
```

Adding a Ticket:
```
# Create new bug interactively
git bug bug new

# Create new bug with title and message
git bug bug new --title "Bug title" --message "Bug description"
```

Managing Ticket Status:
- Simple open/closed status model
```
# Close a ticket
git bug bug status close [BUG_ID]

# Reopen a ticket
git bug bug status open [BUG_ID]
```

Additional Features:
- Labels for categorization
- Comments for discussion
- Web and Terminal UI
- Sync bug data with git remotes

Example Workflow:
```
# List tickets
git bug bug

# Create a new ticket
git bug bug new --title "Fix authentication issue" --message "Users cannot log in"

# Close when resolved
git bug bug status close [ticket-id]
```

Note: The system only supports "open" and "closed" states, with no intermediate statuses built-in.