#!/usr/bin/env bash
# scripts/ec2-sweep.sh [HOURS]
#
# The machines scripts/ec2-run.sh has rented that are still there: each
# one's region, id, type, state and age, in every region of EC2_REGION
# (us-east-1 us-east-2 us-west-2 unless given). They are the instances
# tagged purpose=kani-proofs, and no other is looked at.
#
# With HOURS, those launched longer ago than that are terminated. A
# machine of ec2-run.sh's turns itself off, and so is terminated, after
# EC2_MAX_HOURS (6 unless given); one older than that is one both the
# script and the machine's own clock lost, and is costing its hours.
#
# Needs AWS credentials in the environment, as ec2-run.sh does. Run by
# .github/workflows/proofs-full.yml with harnesses: sweep.
set -euo pipefail

older=${1:-}
regions=${EC2_REGION:-us-east-1 us-east-2 us-west-2}
export AWS_PAGER=""
case "$older" in '' | *[!0-9]*) [ -z "$older" ] || { echo "HOURS: a whole number, not $older" >&2; exit 2; } ;; esac
if [ -z "${AWS_ACCESS_KEY_ID:-}" ] || [ -z "${AWS_SECRET_ACCESS_KEY:-}" ]; then
    echo "no AWS credentials: AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY are not set" >&2
    exit 2
fi

# The repository is public and so is a run's log: the account's number is
# kept out of what AWS says of a refusal.
errors=$(mktemp)
trap 'rm -f "$errors"' EXIT
aws() {
    local status=0
    command aws "$@" 2> "$errors" || status=$?
    sed -E 's/[0-9]{12}/<account>/g' "$errors" >&2
    return $status
}

now=$(date +%s)
there=0
for region in $regions; do
    rows=$(aws ec2 describe-instances --region "$region" \
        --filters Name=tag:purpose,Values=kani-proofs Name=instance-state-name,Values=pending,running,stopping,stopped \
        --query 'Reservations[].Instances[].[InstanceId,InstanceType,State.Name,LaunchTime]' --output text)
    while read -r id type state launched; do
        [ -n "$id" ] || continue
        there=$((there + 1))
        minutes=$(((now - $(date -d "$launched" +%s)) / 60))
        echo "$region $id $type $state, launched $minutes minutes ago"
        if [ -n "$older" ] && [ "$minutes" -gt $((older * 60)) ]; then
            echo "  more than $older hours: terminating it"
            aws ec2 terminate-instances --region "$region" --instance-ids "$id" \
                --query 'TerminatingInstances[0].CurrentState.Name' --output text
        fi
    done <<< "$rows"
done
echo "$there of this script's machines are there"
