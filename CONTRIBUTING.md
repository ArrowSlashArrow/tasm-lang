# Contributing
Thanks for wanting to contribute! All contribution must be made through a pull request. Please see the respective section below for your type of contribution.

## Bug report
- Include a short title, preferably one sentence long, that is informative of the core issue.
- In the description, state exactly what went wrong to the best of your ability.
- If applicable, please provide the program output. 

## Feature request
- Title the issue: "Feature Request: \<your feature>"
- Please provide:
    - a brief summary of the feature
    - what you would like to see from it as a result
    - any necessary details of implementation

## Contribution to TASM compiler
- Title the PR: "\[TASMC] \<brief description>"
- Do not commit large amounts of changes all at once; this makes it harder for maintainers to review your code.
- Use `cargo fmt` to format your code.
- Add any new necessary tests and ensure that `cargo test` passes.
- AI-assisted code is acceptable given that it is fully understood and documented by the programmer; overtly AI-generated code with little thought/understanding behind it is not.
- Please document your code. While this is not a requirement, this makes your code easier to review and maintain.

## Contribution to TASM stdlib
Note: stdlib contributions have stricter requirements due to being a central part of the TASM language. As well as providing core functionality, stdlib code may also serve as an example of good, robust TASM code.
- Title the PR: "\[STDLIB] \<brief description>"
- All contributions to the TASM stdlib *must* be thoroughly documented. Refer to `stdlib/mem_8bit.tasm` as an example.
- Do not use magic group IDs, item IDs, or numeric values in any stdlib file. Any such values must be aliased.
- Do not use any deprecated instructions. Furthermore, do not use `PERS`, `DISPLAY`, or `IOBLOCK`.
- Prefix all internal routine - that is, routines not meant to be used outside of the file - with `_std_{filename}_` to prevent name collisions. This is recommended for aliases too, but is not necessary. 
- If you are unsure whether your contribution fits the criteria, open a PR anyways and a maintainer will help integrate your code into the codebase, provided that it is functional.
- No AI-generated code.

## Other
- Describe your suggestion briefly and informatively in the title
- Provide as much detail as you want in the description, but please provide enough so that the maintainer(s) understand the core of the request.