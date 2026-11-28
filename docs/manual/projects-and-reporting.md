# Projects and reporting

## Save project and Open project

**File > Save project** writes a folder holding four things together: your script, R's objects, the session's log and the package list, with a small `.rcraft` file naming them. If the folder already holds a project, its file names are kept.

**File > Open project** opens that file, on this or another computer: the script comes back in the editor, the objects are loaded into R, the working folder moves to the project's folder, and the package list waits in the [Packages](packages) window to install what is missing. Recent projects are listed under **Open recent project**.

## Save session as

**File > Save session as** writes the commands that ran without error, in order, as one of:

- an R script (`.R`),
- a Quarto document (`.qmd`),
- an R Markdown document (`.Rmd`).

**Save session as Quarto project** does the Quarto form in a folder of its own, with a `_quarto.yml` file. It refuses a folder that already holds one, so nothing is written over.

## The session's report

A report is built up while you work:

- **Reports > Add plot to report** adds the newest plot, or one you name.
- **Reports > Add text to report** adds a paragraph under an optional heading.
- **Add table to report** is on the menu of a data frame in the Objects tab, for a table such as a Model table.
- **Reports > Report contents** lists what is in the report so far, and **Clear report** starts it again; files already saved are not touched.
- **Reports > Save report** writes the report as a Word document (`.docx`) or a web page (`.html`).

## Export table

For a table you want in a paper, open the menu of the data frame in the Objects tab and choose **Export table**. The file's ending chooses the format: `.html`, `.docx`, `.rtf`, `.tex`, `.xlsx` or `.md`. A **Model table** puts several models side by side, with their stars, standard errors and fit statistics, and is the usual thing to export.
