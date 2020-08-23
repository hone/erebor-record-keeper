# Erebor Record Keeper
This is a Discord bot for [The Lord of the Rings: The Card Game](https://www.fantasyflightgames.com/en/products/the-lord-of-the-rings-the-card-game/). It provides a set of commands for finding random quests, organizing events, tracking quests that have been completed.

## Commands

### General
These commands are available at the root level.

#### Quests
Return a set of random quests to pick from the entire quest pool.

Usage:
```
!quest <quantity>
```

If `<quantity>` isn't specified, it defaults to 3.

### Event
These commands are grouped together since they're related to events and have the `event` prefix.

#### Create
Creates a new event by name.

Usage:
```
!event create "<name>"
```

#### Add
Add scenarios to an event.

Usage:
```
!event add
```

#### Set
This sets an event as the active event for users.

Usage:
```
!event set
```

#### Archive
Archive an event once it's over.

Usage:
```
!event archive
```

#### Quest
List out a set number quests to do associated with the active event.

Usage:
```
!event quest <quantity>
```

If `<quantity>` isn't specified, it defaults to 3.

#### Complete
Mark a quest as complete for the event by the scenario code.

Usage:
```
!event complete <code>
```

#### Checkout
Reserve a quest for 2 hours. These quests won't show up in the `!event quest` command.

Usage:
```
!event checkout <code>
```

#### Progress
Display how much of the event quests are complete

Usage:
```
!event progress
```
