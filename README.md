`jp` is a minimal version control system (VCS) based on the same ideas as
[`pijul`](https://pijul.com), as described in the series of blog posts
[here](https://jneem.github.io). This is not a real VCS, and you should not use
it for anything important. (For starters, it is only capable of tracking a
single file.) I wrote `jp` to help me understand the ideas discussed in the
blog posts, and I'm making it public in the hope that maybe it will help
someone else also.

# Installation

`jp` is a command line program. It has only been tested on Linux, although it
will probably also work on similar operating systems.
`jp` is written in [rust](https://rust-lang.org); to install it, you will need
a rust toolchain installed. Once you've done that, clone this repository and
build with `cargo`:

```
$ git clone https://github.com/jneem/jp.git
$ cd jp
$ cargo build --release
```

Then you can find the `jp` binary in the `target/release/` directory.

# Usage

## Creating a repository

To start your `jp` journey, initialize a repository in the current directory:
```
$ jp init
Created empty jp repository.
```

This will create a `.jp` directory in the current directory, containing the
file `db`. This `db` file contains all of `jp`'s internal data. It's plain text
(in YAML format), so you can look if you're curious.

## Creating and applying patches

Each `jp` repository is capable of tracking only one file, and the filename
defaults to `jp_file.txt`. To create a patch, use the command `jp patch create`.
For example, let's suppose that you've just created a repository, and then you edit the
file `jp_file.txt`. To create a patch reflecting your new changes: do
```
$ jp patch create --author "My Name" --description "Something something"
Created patch rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=
```
That long string in the output is the unique identifier of the patch you just created.
It was obtained by hashing the contents of the patch (including a timestamp, so you're
unlikely to see the same hash twice even if you have exactly the same contents).

One idiosyncracy of `jp` is that it doesn't (by default) *apply* the patches as soon
as you create them, as opposed to (for example) `git commit`, which creates a patch
and also applies it to the current branch. If you want to both create and apply a patch
at the same time, provide the argument `--then-apply` to `jp patch create`:
```
$ jp patch create --author "My Name" --description "Something something" --then-apply
Created and applied patch rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=
```

Alternatively, you can first create the patch and then apply it with the `jp patch apply` command:
```
$ jp patch create --author "My Name" --description "Something something"
Created patch rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=
$ jp patch apply rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=
Applied:
  vDLmQ2m8JnblI0wPq2bTSYusqNtHOLNo1iRt4nWdyLY=
```

If you want to unapply a patch, use `jp patch apply --revert`:
```
$ jp patch apply -R rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=
Unapplied:
  vDLmQ2m8JnblI0wPq2bTSYusqNtHOLNo1iRt4nWdyLY=
```

## Outputting a file

Another of `jp`'s quirks is that it doesn't automatically update the working
copy of your file to match changes in the internal repository. To output a file
containing the repository's current contents, use the `jp render` command. By default,
the repository's contents will be outputted to the `jp_file.txt` file, but you can
change that.
```
$ jp render # outputs the repository contents to jp_file.txt
$ jp render --path other_file.txt # specify another output path
```

## Putting it together

```
$ jp init
Created empty jp repository.
$ echo "First line" > jp_file.txt
$ jp patch create --author Me --description "I wuz here" --then-apply
Created and applied patch rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=

# Now the file stored in the repository consists of the single line
# "First line". The file jp_file.txt also consists of the single line
# "First line", but that's because we put it there ourselves; jp hasn't
# touched it.

$ echo "Second line" >> jp_file.txt
$ jp patch create --author Me --description "Me again" --then-apply
Created and applied patch xGRnP1j1o9FdJPPJoD6OM4Pxj3qgyN2hKG_0qg54t38=

# Now the file stored in the repository has two lines, and so does the
# file jp_file.txt.

$ jp patch apply --reverse
Unapplied:
  xGRnP1j1o9FdJPPJoD6OM4Pxj3qgyN2hKG_0qg54t38=

# The file stored in the repository has just one line. To reflect that change
# in the filesystem, we need to render:
$ jp render
Successfully wrote file 'jp_file.txt'
$ cat jp_file.txt
First line
```
