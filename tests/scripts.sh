#!/usr/bin/env bash
# Exercises the two shell scripts of the projet : syntax, guards, and for the
# server the address it prints, which has to be copiable as it stands
set -euo pipefail

racine="$(cd "$(dirname "$0")/.." && pwd)"
rates=0

verifier()
{
    libelle="$1"
    shift
    if "$@" > /dev/null 2>&1; then
        printf '  ok   %s\n' "$libelle"
    else
        printf '  RATE %s\n' "$libelle"
        rates=$((rates + 1))
    fi
}

sans()
{
    ! grep -q "$1" "$2"
}

repond()
{
    for _ in $(seq 1 40); do
        if curl -fs -o /dev/null "$1"; then
            return 0
        fi
        sleep 0.25
    done
    return 1
}

for script in construire.sh servir.sh; do
    chemin="$racine/$script"
    verifier "$script se parse" bash -n "$chemin"
    verifier "$script s'arrête à la première erreur" grep -q 'set -euo pipefail' "$chemin"
    verifier "$script est exécutable" test -x "$chemin"
done

# The address came out as localhost :8000 for a while, the french spacing of
# a log applied to a port. Nobody could paste it
port=8771
sortie="$(mktemp)"
"$racine/servir.sh" "$port" > "$sortie" 2>&1 &
serveur=$!
trap 'kill "$serveur" 2>/dev/null || true; rm -f "$sortie"' EXIT

verifier "servir.sh rend la page" repond "http://127.0.0.1:$port/web/index.html"
verifier "l'adresse imprimée se colle telle quelle" \
    grep -q "http://localhost:$port/web/index.html" "$sortie"
verifier "aucune espace avant le port" sans 'localhost :' "$sortie"

printf '%d contrôle(s) en échec\n' "$rates"
if [ "$rates" -gt 0 ]; then
    exit 1
fi
exit 0
