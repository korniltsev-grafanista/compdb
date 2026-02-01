write a rust program to modify-filter compilation_database.json file. The goal
  is to make the compilation database smaller by removing some src files from it.
  accept a pth to copile_commands.json, or assume ./compile_commands.json if
  nothing passed. the very first thing the program should do is backup the
  original one to an adjacent files without overwriting any previous backups. it
  should accept regular expressions to either include or exclude some files,
  exclude should exclude but include should include something even if its
  excluded. at the end print some stags - number of original files and number of
  processed files in the compdb. heres an example of compdb file
  @/home/korniltsev/oss/linux/compile_commands.json
