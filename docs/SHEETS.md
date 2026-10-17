# Zenocast Spreadsheet Data Synchronization

Zenocast features direct integration with local and cloud spreadsheets (CSV, Microsoft Excel `.xlsx`, and Google Sheets). Tournament producers can maintain match scores, caster names, schedule tickers, and player stats in a shared spreadsheet, and Zenocast will reflect updates on air in real-time.

---

## 1. Supported Spreadsheet Sources

1. **Local CSV / TSV**: Fast, file-watcher or polling backed local text files.
2. **Microsoft Excel (`.xlsx`)**: Local workbook files created and edited in Excel. Zenocast reads them using non-blocking file-sharing flags so Excel can keep the workbook open without locking Zenocast out.
3. **Google Sheets**:
   - **Published CSV Feed**: Instant synchronization via published Google Sheets CSV URL.
   - **Google Sheets API**: Direct integration via OAuth or service accounts.

---

## 2. Cell Addressing & Binding

Spreadsheet cells are bound to Zenocast `DataStore` reactive variables using standard alphanumeric coordinate notation:

### 2.1 Single Cell Binding

Binds an individual cell directly to a typed `DataStore` variable:

| Coordinate | Variable Key | Variable Type | Example Value |
| :--- | :--- | :--- | :--- |
| `B2` | `scoreboard.team1.score` | `Number` | `13` |
| `B3` | `scoreboard.team2.score` | `Number` | `11` |
| `A2` | `scoreboard.team1.name` | `String` | `"Sentinels"` |
| `A3` | `scoreboard.team2.name` | `String` | `"Fnatic"` |
| `C1` | `match.active_map` | `String` | `"Bind"` |

### 2.2 Range Binding to Repeater Layers

Binds a 2D cell range to a `Repeater` layer to dynamically generate leaderboards or roster player cards:

```rust
link.add_binding(CellBinding::new_range(
    "roster.players",
    CellTarget::Range("A2:C6".to_string()),
    vec!["name", "role", "kd_ratio"],
));
```

The Repeater layer instantiates 5 copies of the player card template, offsetting each by `(0.0, 64.0)` px, and populates sub-layer text elements from the row columns.

---

## 3. Reactive Data Triggers & State Machines

When a cell value changes in the spreadsheet:
1. `SheetSyncEngine` detects the difference and updates the `DataStore`.
2. `DataStore` notifies registered listeners and state machines.
3. Configured `DataTriggerRule` transitions the overlay into a reactive animation (e.g., score increment triggers a `score_change` flash / pulse animation).

---

## 4. Fault Tolerance & Graceful Degradation

- **File Locks (Excel Saving)**: If Excel holds a momentary write lock, Zenocast retains the last known good value in memory and flags the link with `SheetLinkStatus::Warning`. It never crashes, panics, or blinks blank text onto air.
- **Network Outages**: If internet connectivity is interrupted while syncing Google Sheets, Zenocast serves cached values indefinitely until connectivity resumes.
- **Type Coercion**: Automatically coerces numeric strings (`"42"`), booleans (`"TRUE"`, `"1"`), and floats smoothly.
