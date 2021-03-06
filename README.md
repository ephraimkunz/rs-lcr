![CI](https://github.com/ephraimkunz/rs-lcr/workflows/CI/badge.svg?branch=master)
![Dependabot](https://flat.badgen.net/dependabot/ephraimkunz/rs-lcr?icon=dependabot)
[![codecov](https://codecov.io/gh/ephraimkunz/rs-lcr/branch/master/graph/badge.svg?token=DRXPYPQIP6)](undefined)
# rs-lcr

Unofficial API and utilities for Leader and Clerk Resources (LCR) data in Rust. 

The [official Church LCR website](https://lcr.churchofjesuschrist.org) is where leaders and clerks spend a lot of time. The Church doesn't offer an API, so I thought I'd expose one. 
* A Python package to do similar things exists [here](https://github.com/philipbl/LCR-API) and I've used it for a few projects, but the Church keeps changing the login flow so it's constantly broken. This project uses headless Chrome to login so I'm hoping it will be more stable, and easier to fix if things do change.

* Current supported:
  - [x] Member list
  - [x] Members moved in
  - [x] Members moved out
  - [x] Member profile
