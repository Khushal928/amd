# amFOSS Daemon

A discord bot designed specifically for the amFOSS server, to automate chores like role assignment, lab attendance and status update tracking. `amD` does not have a database of it's own, and relies on [Root.](https://www.github.com/amfoss/amd)

# Running your own instance

`amD` is tailored for the amFOSS server and as such, is not really a project worth forking for your own needs. There are many other alternative templates for Discord bots you could use instead.

If you want to contribute to `amD`, you'll likely need to run your own instance to test your contributions. To compile from source, you'll need:

- [Rust](https://www.rust-lang.org/tools/install)
- A Discord Bot Token from the [Discord Developer Protal](https://discord.com/developers/) .

After which, you can make your changes to the source code and modify the environment variables to have your own instance up and running. A more detailed guide to development and contributing can be found in [CONTRIBUTING.md.](/docs/CONTRIBUTING.md)

# License
This project is licensed under the GNU General Public License v3.0. See the LICENSE file for details.
