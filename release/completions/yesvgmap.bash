_basher___yesvgmap() {
	local cur prev opts
	COMPREPLY=()
	cur="${COMP_WORDS[COMP_CWORD]}"
	prev="${COMP_WORDS[COMP_CWORD-1]}"
	opts=()
	if [[ ! " ${COMP_LINE} " =~ " -h " ]] && [[ ! " ${COMP_LINE} " =~ " --help " ]]; then
		opts+=("-h")
		opts+=("--help")
	fi
	if [[ ! " ${COMP_LINE} " =~ " -V " ]] && [[ ! " ${COMP_LINE} " =~ " --version " ]]; then
		opts+=("-V")
		opts+=("--version")
	fi
	if [[ ! " ${COMP_LINE} " =~ " -a " ]] && [[ ! " ${COMP_LINE} " =~ " --attribute " ]]; then
		opts+=("-a")
		opts+=("--attribute")
	fi
	if [[ ! " ${COMP_LINE} " =~ " -l " ]] && [[ ! " ${COMP_LINE} " =~ " --list " ]]; then
		opts+=("-l")
		opts+=("--list")
	fi
	if [[ ! " ${COMP_LINE} " =~ " -o " ]] && [[ ! " ${COMP_LINE} " =~ " --output " ]]; then
		opts+=("-o")
		opts+=("--output")
	fi
	if [[ ! " ${COMP_LINE} " =~ " -p " ]] && [[ ! " ${COMP_LINE} " =~ " --prefix " ]]; then
		opts+=("-p")
		opts+=("--prefix")
	fi
	opts=" ${opts[@]} "
	if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
		COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
		return 0
	fi
	case "${prev}" in
		--list|--output|-l|-o)
			if [ -z "$( declare -f _filedir )" ]; then
				COMPREPLY=( $( compgen -f "${cur}" ) )
			else
				COMPREPLY=( $( _filedir ) )
			fi
			return 0
			;;
		*)
			COMPREPLY=()
			;;
	esac
	COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
	return 0
}
complete -F _basher___yesvgmap -o bashdefault -o default yesvgmap
