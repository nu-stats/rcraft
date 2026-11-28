# The menus

Every dialog in Data, Statistics, Graphs and Reports works the same way: you choose columns and options, the dialog shows the R it will write, and nothing runs until you press OK. A dialog that needs a data frame is greyed out until one is loaded. Each dialog has a Help button with its own page.

## File

Bringing data in and taking work out.

- **Use my file** reads a data file. **Example data** loads a practice data frame from R's own datasets.
- **Run do-file** runs a Stata do-file on R. See [Stata mode](command-window).
- **Open script**, **Save script** and **Save script as** work on the script editor.
- **Save session as** writes what ran without error as an R script, a Quarto document or an R Markdown document. **Save project** and **Open project** keep the script, the objects, the log and the package list together. See [projects and reporting](projects-and-reporting).
- Further down are the entries for writing the data frame to a text file, an Excel workbook, a Stata, SPSS or SAS file, an R data file or Parquet, and for saving the whole workspace.

## Edit

Clear the Results, copy the session log, or copy the session as a script. **Accent** and **Appearance** change the colours: light, dark, or the system's setting.

## Data

Working on the data frame: new variables, recoding, changing a variable's type, text and dates, filtering and sorting rows, labels, joining and appending data frames, aggregating, reshaping wide to long and back, duplicates, random samples and dropping missing values.

## Statistics

Summaries, tests and models, from a simple table of means to regression, survival, panel data, time series and more.

## Graphs

Plots drawn from your data. **Export plot** saves the plot on show to a file, and is greyed out until a plot has been drawn.

## Reports

Collects tables, plots and text from the session into one report. See [projects and reporting](projects-and-reporting).

## Packages

Finding, installing and updating the R packages that some dialogs need. See [packages](packages).

## Session

- **Stop the running command** interrupts R.
- **Restart R** starts a fresh R process.
- **Working folder** chooses the folder R reads and writes files in.
- **New R session** opens a second, separate R session. The sessions are listed here to switch between, and you can rename or close them.

## Help

- **RCraft manual** opens this manual.
- **Search R help** opens R's own help pages for the packages installed here.
- **Vignettes and package manuals** lists the long-form guides that packages ship with.
- **Stata and R: what differs** explains where R's results differ from Stata's, command by command.
