`ojo` is a minimal version control system (VCS) based on the same ideas as
[`pijul`](https://pijul.com), as described in the series of blog posts
[here](https://jneem.github.io). This is not a real VCS, and you should not use
it for anything important. (For starters, it is only capable of tracking a
single file.) I wrote `ojo` to help me understand the ideas discussed in the
blog posts, and I'm making it public in the hope that maybe it will help
someone else also.

# Installation

`ojo` is a command line program. It has only been tested on Linux, although it
will probably also work on similar operating systems.
`ojo` is written in [rust](https://rust-lang.org); to install it, you will need
a rust toolchain installed. Once you've done that, clone this repository and
build with `cargo`:

```
$ git clone https://github.com/jneem/ojo.git
$ cd ojo
$ cargo build --release
```

Then you can find the `ojo` binary in the `target/release/` directory.

# Usage

## Creating a repository

To start your `ojo` journey, initialize a repository in the current directory:
```
$ ojo init
Created empty ojo repository.
```

This will create a `.ojo` directory in the current directory, containing the
file `db`. This `db` file contains all of `ojo`'s internal data. It's plain text
(in YAML format), so you can look if you're curious.

## Creating and applying patches

Each `ojo` repository is capable of tracking only one file, and the filename
defaults to `ojo_file.txt`. To create a patch, use the command `ojo patch create`.
For example, let's suppose that you've just created a repository, and then you edit the
file `ojo_file.txt`. To create a patch reflecting your new changes: do
```
$ ojo patch create --author "My Name" --description "Something something"
Created patch rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=
```
That long string in the output is the unique identifier of the patch you just created.
It was obtained by hashing the contents of the patch (including a timestamp, so you're
unlikely to see the same hash twice even if you have exactly the same contents).

One idiosyncracy of `ojo` is that it doesn't (by default) *apply* the patches as soon
as you create them, as opposed to (for example) `git commit`, which creates a patch
and also applies it to the current branch. If you want to both create and apply a patch
at the same time, provide the argument `--then-apply` to `ojo patch create`:
```
$ ojo patch create --author "My Name" --description "Something something" --then-apply
Created and applied patch rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=
```

Alternatively, you can first create the patch and then apply it with the `ojo patch apply` command:
```
$ ojo patch create --author "My Name" --description "Something something"
Created patch rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=
$ ojo patch apply rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=
Applied:
  vDLmQ2m8JnblI0wPq2bTSYusqNtHOLNo1iRt4nWdyLY=
```

If you want to unapply a patch, use `ojo patch apply --revert`:
```
$ ojo patch apply -R rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=
Unapplied:
  vDLmQ2m8JnblI0wPq2bTSYusqNtHOLNo1iRt4nWdyLY=
```

## Outputting a file

Another of `ojo`'s quirks is that it doesn't automatically update the working
copy of your file to match changes in the internal repository. To output a file
containing the repository's current contents, use the `ojo render` command. By default,
the repository's contents will be outputted to the `ojo_file.txt` file, but you can
change that.
```
$ ojo render # outputs the repository contents to ojo_file.txt
$ ojo render --path other_file.txt # specify another output path
```

## Putting it together

```
$ ojo init
Created empty ojo repository.
$ echo "First line" > ojo_file.txt
$ ojo patch create --author Me --description "I wuz here" --then-apply
Created and applied patch rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=

# Now the file stored in the repository consists of the single line
# "First line". The file ojo_file.txt also consists of the single line
# "First line", but that's because we put it there ourselves; ojo hasn't
# touched it.

$ echo "Second line" >> ojo_file.txt
$ ojo patch create --author Me --description "Me again" --then-apply
Created and applied patch xGRnP1j1o9FdJPPJoD6OM4Pxj3qgyN2hKG_0qg54t38=

# Now the file stored in the repository has two lines, and so does the
# file ojo_file.txt.

$ ojo patch apply --reverse
Unapplied:
  xGRnP1j1o9FdJPPJoD6OM4Pxj3qgyN2hKG_0qg54t38=

# The file stored in the repository has just one line. To reflect that change
# in the filesystem, we need to render:
$ ojo render
Successfully wrote file 'ojo_file.txt'
$ cat ojo_file.txt
First line
```

## Conflicts and resolution

The basic theory behind the way that `ojo` deals with conflicts is described
in the series of blog posts [here](https://jneem.github.io/). The main idea
is that instead of files, `ojo` stores "graggles", which are directed graphs
of lines. A file is the special case of a graggle in which the directed graph
of lines enforces a unique ordering. (More precisely, a graggle is a file if
it has a unique topological sort.) Most of the time, you want your repository
to represent a file; if it reprsents a graggle that isn't a file, we call it
a conflict.

Here's a way to get a conflict:
```
$ ojo init
Created empty ojo repository.
$ echo "First line" > ojo_file.txt
$ ojo patch create --author Me --description "Starting out" --then-apply
Created and applied patch rLbZ6RjMol8_wV0tW2dnMapcaNVJB25A9uWFXixDU6c=
$ echo "Second line" >> ojo_file.txt
$ ojo patch create --author Me --description "Working hard"
Created patch xGRnP1j1o9FdJPPJoD6OM4Pxj3qgyN2hKG_0qg54t38=

# Notice that we haven't applied the second patch, so the file in the repository
# only has the first patch applied. Now let's edit the file on disk so that
# it consists of "First line" followed by "Alternate second line":

$ echo "First line" > ojo_file.txt
$ echo "Alternate second line" >> ojo_file.txt
$ ojo patch create --author Me --description "Working differently" --then-apply
Created and applied patch y-lgpjY30n5STzqtrMOEkvBM_WUWy0Yji91y9KTzptc=

# And finally, we apply the patch that added "Second line"
$ ojo patch apply xGRnP1j1o9FdJPPJoD6OM4Pxj3qgyN2hKG_0qg54t38=
Applied:
  xGRnP1j1o9FdJPPJoD6OM4Pxj3qgyN2hKG_0qg54t38=
```

Now, the effect of that long command listing was to create a graggle containing
the line "First line" followed by either "Second line" or "Alternate second line",
but with no prescribed order between the two possible second lines. In particular,
the result isn't a file, because the lines it contains aren't in a linear order.
If you try to render the file, it won't work:
```
$ ojo render
Error: Couldn't render a file, because the data isn't ordered
```

There are two important commands that you can use to resolve a conflict. The first
is to inspect it, by rendering a graph:
```
$ ojo graph
```
This will create a "dot" file, which can be rendered using
[graphviz](https://www.graphviz.org):
```
$ dot -o out.pdf -Tpdf out.dot
```
And now you can look at `out.pdf` to see a visualization of your graggle that isn't
a file.

Once you understand what's going on, you can resolve your conflict using `ojo`'s
built-in interactive graggle resolver:
```
$ ojo resolve --author Me
```
This interactive utility will guide you through the process of turning the
unordered graggle into a totally ordered file. In the example above, this
amounts to deciding whether "Second line" should go before or after "Alternate
second line". (At some point in the probably-distant future, I hope to create
some comprehensive documentation for `ojo resolve`'s user interface. But for
now, hopefully it's reasonably explorable. Anyway, you can see all of the
currently-active key bindings in the top right.)

When `ojo resolve` is done, it will produce a patch, which you can then apply
to get rid of the conflict:
```
$ ojo resolve --author Me
# do the interactive thing...
Created patch SfxSwnA2POPHzL4eNNHku7t4Lyl5xW7Ge9pRXr5hV60=
$ ojo patch apply SfxSwnA2POPHzL4eNNHku7t4Lyl5xW7Ge9pRXr5hV60=
Applied:
  SfxSwnA2POPHzL4eNNHku7t4Lyl5xW7Ge9pRXr5hV60=
$ ojo render
Successfully wrote file 'ojo_file.txt'
```
