/*
Copyright 2011-2013 Paul Ruane.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

package cli

type Command interface {
	Name() CommandName
	Synopsis() string
	Description() string
	Options() Options
	Exec(options Options, args []string) error
}

func LookupOption(command Command, name string) *Option {
	for _, option := range globalOptions {
		if option.LongName == name || option.ShortName == name {
			return &option
		}
	}

	if command != nil {
		for _, option := range command.Options() {
			if option.LongName == name || option.ShortName == name {
				return &option
			}
		}
	}

	return nil
}

var globalOptions = Options{Option{"-v", "--verbose", "show verbose messages"},
	Option{"-h", "--help", "show help and exit"},
	Option{"-V", "--version", "show version information and exit"}}