#!/usr/bin/env bash
# scripts/ec2-run.sh COMMAND
#
# Rent one machine from AWS EC2, run COMMAND on it in a checkout of this
# repository, bring logs/kani back, and give the machine back. For the
# proofs a GitHub runner has not the memory or the cores for: run by
# .github/workflows/proofs-full.yml (machine: aws), or by hand with AWS
# credentials in the environment.
#
#   EC2_TYPE       the instance type (r7a.8xlarge: 32 cores, 256 GB)
#   EC2_MARKET     spot (the default) or on-demand
#   EC2_REGION     us-east-1
#   EC2_DISK_GB    500
#   EC2_SETUP      kani (Rust and Kani installed before COMMAND, the
#                  default) or none
#   EC2_MAX_HOURS  6: the machine turns itself off after this long whatever
#                  becomes of this script, and a machine turned off is one
#                  given back (it is launched to terminate on shutdown)
#   EC2_WAIT_MINUTES  320: how long to wait for COMMAND; a GitHub job has
#                  six hours
#   EC2_SSH_FROM   the address port 22 is opened to: this machine's, as
#                  the world sees it, unless given
#
# What it costs is the machine's hours, to the minute it is terminated;
# nothing is left behind but a security group with no rule in it.
#
# What it needs of the account: to describe, run and tag instances, to
# terminate the ones tagged purpose=kani-proofs, to make a security group
# and open and close its port 22, and, for a first Spot request, to create
# the Spot service's role. Nothing else: the machine is reached over SSH
# with a key made for the run, opened to this machine's address only, and
# reads the repository as anyone can (it is public).
#
# The exit status is COMMAND's; 3 if the machine went away under it (a
# Spot machine reclaimed), 4 if COMMAND was still running when the wait
# ran out. logs/kani is brought back every five minutes as well as at the
# end, so a machine that goes away takes at most five minutes of results.
set -euo pipefail

cd "$(dirname "$0")/.."

command=${1:?usage: scripts/ec2-run.sh COMMAND}
type=${EC2_TYPE:-r7a.8xlarge}
market=${EC2_MARKET:-spot}
disk=${EC2_DISK_GB:-500}
setup=${EC2_SETUP:-kani}
max_hours=${EC2_MAX_HOURS:-6}
wait_minutes=${EC2_WAIT_MINUTES:-320}
export AWS_DEFAULT_REGION=${EC2_REGION:-us-east-1} AWS_PAGER=""
repo=${GITHUB_SERVER_URL:-https://github.com}/${GITHUB_REPOSITORY:?GITHUB_REPOSITORY: owner/name of the repository to check out}
sha=${GITHUB_SHA:-$(git rev-parse HEAD)}

case "$market" in spot | on-demand) ;; *) echo "EC2_MARKET: spot or on-demand, not $market" >&2; exit 2 ;; esac
case "$setup" in kani | none) ;; *) echo "EC2_SETUP: kani or none, not $setup" >&2; exit 2 ;; esac
if [ -z "${AWS_ACCESS_KEY_ID:-}" ] || [ -z "${AWS_SECRET_ACCESS_KEY:-}" ]; then
    echo "no AWS credentials: AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY are not set" >&2
    echo "(in CI they are the repository's Actions secrets of those names)" >&2
    exit 2
fi

work=$(mktemp -d)
id="" group="" me=""
give_back() {
    status=$?
    set +e
    if [ -n "$id" ]; then
        echo "terminating $id"
        aws ec2 terminate-instances --instance-ids "$id" --query 'TerminatingInstances[0].CurrentState.Name' --output text \
            || echo "COULD NOT TERMINATE $id: it turns itself off within $max_hours hours of its launch; terminate it in the console" >&2
    fi
    if [ -n "$group" ] && [ -n "$me" ]; then
        aws ec2 revoke-security-group-ingress --group-id "$group" --protocol tcp --port 22 --cidr "$me/32" > /dev/null
    fi
    rm -rf "$work"
    exit $status
}
trap give_back EXIT

# This repository is public and so is a run's log: the account's number
# is kept out of what AWS says of a refusal, and no address is printed.
aws() { command aws "$@" 2> >(sed -E 's/[0-9]{12}/<account>/g' >&2); }

aws sts get-caller-identity --query UserId --output text > /dev/null
echo "the AWS credentials are accepted"

# The newest Ubuntu 24.04 that Canonical publishes, and its root device.
found=$(aws ec2 describe-images --owners 099720109477 \
    --filters 'Name=name,Values=ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-*' 'Name=state,Values=available' \
    --query 'sort_by(Images,&CreationDate)[-1].[ImageId,RootDeviceName]' --output text)
read -r image root <<< "$found" || true
case "${image:-}" in ami-*) ;; *) echo "no Ubuntu 24.04 image found in $AWS_DEFAULT_REGION" >&2; exit 2 ;; esac

# A key for this run only, given to the machine as it boots; and the hour
# it turns itself off.
ssh-keygen -q -t ed25519 -N '' -C kani-proofs -f "$work/key"
cat > "$work/user-data" <<EOF
#cloud-config
ssh_authorized_keys:
  - $(cat "$work/key.pub")
runcmd:
  - shutdown -h +$((max_hours * 60))
EOF

# One security group, kept between runs with no rule in it; port 22 is
# opened to this machine's address for the run and closed after.
vpc=$(aws ec2 describe-vpcs --filters Name=is-default,Values=true --query 'Vpcs[0].VpcId' --output text)
[ "$vpc" != None ] || { echo "the account has no default VPC in $AWS_DEFAULT_REGION" >&2; exit 2; }
group=$(aws ec2 describe-security-groups --filters Name=group-name,Values=kani-proofs "Name=vpc-id,Values=$vpc" \
    --query 'SecurityGroups[0].GroupId' --output text)
if [ "$group" = None ]; then
    group=$(aws ec2 create-security-group --group-name kani-proofs --vpc-id "$vpc" \
        --description "SSH to the Kani proof machines, opened for a run and closed after" --query GroupId --output text)
fi
me=${EC2_SSH_FROM:-$(curl -fsS https://checkip.amazonaws.com)}
aws ec2 authorize-security-group-ingress --group-id "$group" --protocol tcp --port 22 --cidr "$me/32" > /dev/null 2> "$work/ingress.err" \
    || grep -q InvalidPermission.Duplicate "$work/ingress.err" || { cat "$work/ingress.err" >&2; exit 2; }

# The default subnets of the zones that offer the type, tried in turn: a
# zone may have none of it to spare, above all on the Spot market.
zones=$(aws ec2 describe-instance-type-offerings --location-type availability-zone \
    --filters "Name=instance-type,Values=$type" --query 'InstanceTypeOfferings[].Location' --output text | tr '\t' ',')
[ -n "$zones" ] || { echo "$type is not offered in $AWS_DEFAULT_REGION" >&2; exit 2; }
subnets=$(aws ec2 describe-subnets --filters "Name=vpc-id,Values=$vpc" Name=default-for-az,Values=true \
    "Name=availability-zone,Values=$zones" --query 'Subnets[].SubnetId' --output text)
[ -n "$subnets" ] || { echo "no default subnet in a zone that offers $type" >&2; exit 2; }

launch=(--image-id "$image" --instance-type "$type" --count 1 --security-group-ids "$group"
        --instance-initiated-shutdown-behavior terminate --user-data "file://$work/user-data"
        --block-device-mappings "DeviceName=$root,Ebs={VolumeSize=$disk,VolumeType=gp3,Iops=6000,Throughput=500,DeleteOnTermination=true}"
        --tag-specifications "ResourceType=instance,Tags=[{Key=purpose,Value=kani-proofs},{Key=Name,Value=kani-proofs-${GITHUB_RUN_ID:-by-hand}}]"
                             "ResourceType=volume,Tags=[{Key=purpose,Value=kani-proofs}]")
if [ "$market" = spot ]; then
    launch+=(--instance-market-options 'MarketType=spot,SpotOptions={SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}')
fi
for subnet in $subnets; do
    if id=$(aws ec2 run-instances "${launch[@]}" --subnet-id "$subnet" --query 'Instances[0].InstanceId' --output text 2> "$work/launch.err"); then
        break
    fi
    id=""
    cat "$work/launch.err" >&2
    # Asking another zone does not change what the account may have.
    if grep -Eq 'MaxSpotInstanceCountExceeded|VcpuLimitExceeded|UnauthorizedOperation|PendingVerification|OptInRequired' "$work/launch.err"; then
        break
    fi
done
if [ -z "$id" ]; then
    echo "no $market $type was to be had in $AWS_DEFAULT_REGION (the refusals are above)" >&2
    if grep -Eq 'MaxSpotInstanceCountExceeded|VcpuLimitExceeded' "$work/launch.err"; then
        echo "the account's vCPU quota is below this machine's: Service Quotas, EC2," >&2
        echo "'All Standard (A, C, D, H, I, M, R, T, Z) Spot Instance Requests' for Spot," >&2
        echo "'Running On-Demand Standard (A, C, D, H, I, M, R, T, Z) instances' otherwise" >&2
    fi
    exit 2
fi
echo "launched $id: $market $type, $disk GB, in $AWS_DEFAULT_REGION; it turns itself off in $max_hours hours"

aws ec2 wait instance-running --instance-ids "$id"
address=$(aws ec2 describe-instances --instance-ids "$id" --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)
case "$address" in [0-9]*) ;; *) echo "$id has no public address" >&2; exit 2 ;; esac
there=(-i "$work/key" -o StrictHostKeyChecking=accept-new -o "UserKnownHostsFile=$work/known_hosts" -o BatchMode=yes
       -o ConnectTimeout=10 -o ServerAliveInterval=30 -o LogLevel=ERROR)
on_it() { ssh "${there[@]}" "ubuntu@$address" "$@"; }
reached=""
for _ in $(seq 60); do
    if on_it true 2> /dev/null; then reached=yes; break; fi
    sleep 5
done
[ -n "$reached" ] || { echo "$id did not answer on port 22 in ten minutes" >&2; exit 2; }
on_it 'cloud-init status --wait > /dev/null 2>&1; echo "$(nproc) cores, $(free -g | awk "NR==2{print \$2}") GB, $(df -h --output=avail / | tail -1 | tr -d " ") of disk free"'

# The checkout, by the commit: the repository is public, so the machine
# holds no credential of any kind.
on_it "git init -q repo && cd repo && git fetch -q --depth 1 '$repo' '$sha' && git checkout -q FETCH_HEAD && git log --oneline -1"
if [ "$setup" = kani ]; then
    on_it 'bash -s' <<'EOF'
set -euo pipefail
{
    export DEBIAN_FRONTEND=noninteractive
    sudo apt-get update -q
    sudo apt-get install -y -q build-essential
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    . "$HOME/.cargo/env"
    cd repo
    cargo install --locked kani-verifier --version "$(sed -n 's/^KANI_VERSION=//p' scripts/prove.sh)"
    cargo kani setup
} > ~/setup.log 2>&1 || { tail -n 40 ~/setup.log; echo "the machine's setup failed (above)"; exit 1; }
. "$HOME/.cargo/env"
cargo kani --version
EOF
fi

# COMMAND, on its own there: this script only watches, so that a
# connection lost is not a run lost.
printf '%s\n' "$command" | on_it 'cat > run.sh'
on_it 'cd repo && nohup setsid bash -c "[ -f ~/.cargo/env ] && . ~/.cargo/env; bash ~/run.sh; echo \$? > ~/run.exit" > ~/run.log 2>&1 < /dev/null &'
echo "running: $command"

bring_back() {
    mkdir -p logs/kani
    on_it 'cd repo && [ -d logs/kani ] && tar -cz --exclude="*.out*" logs/kani 2> /dev/null' > "$work/logs.tgz" 2> /dev/null \
        && [ -s "$work/logs.tgz" ] && tar -xzf "$work/logs.tgz"
}

deadline=$(($(date +%s) + wait_minutes * 60))
lost=0 minute=0
while :; do
    sleep 60
    minute=$((minute + 1))
    if state=$(on_it 'cat run.exit 2> /dev/null || echo running' 2> /dev/null); then
        lost=0
    else
        lost=$((lost + 1))
        if [ $lost -ge 5 ]; then
            echo "the machine has not answered for five minutes: it is $(aws ec2 describe-instances --instance-ids "$id" \
                --query 'Reservations[0].Instances[0].[State.Name,StateReason.Message]' --output text 2>&1 | tr '\t' ' ')" >&2
            exit 3
        fi
        continue
    fi
    # Every five minutes, and at the end: the results so far, here, where
    # a machine reclaimed does not take them with it.
    if [ "$state" != running ] || [ $((minute % 5)) -eq 0 ]; then
        bring_back || true
        echo "minute $minute: $(cat logs/kani/shard-*.txt 2> /dev/null | awk '{n[$1]++} END {for (k in n) printf "%d %s  ", n[k], k}')"
    fi
    if [ "$state" != running ]; then
        on_it 'tail -n 60 run.log'
        echo "COMMAND ended with status $state after $minute minutes"
        exit "$state"
    fi
    if [ "$(date +%s)" -ge $deadline ]; then
        on_it 'tail -n 20 run.log' || true
        echo "COMMAND was still running after $wait_minutes minutes; its results so far are kept" >&2
        exit 4
    fi
done
