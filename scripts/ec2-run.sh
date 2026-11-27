#!/usr/bin/env bash
# scripts/ec2-run.sh COMMAND
#
# Rent one machine from AWS EC2, run COMMAND on it in a checkout of this
# repository, bring logs/kani back, and give the machine back. For the
# proofs a GitHub runner has not the memory or the cores for: run by
# .github/workflows/proofs-full.yml (machine: aws), or by hand with AWS
# credentials in the environment.
#
#   EC2_TYPE       the instance types, tried in order (r7a.8xlarge: 32
#                  cores, 256 GB)
#   EC2_MARKET     spot (the default) or on-demand
#   EC2_MAX_PRICE  1.25: the most a Spot machine may cost, dollars an hour
#   EC2_REGION     the regions, tried in order (us-east-1)
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
#   EC2_ATTEMPTS   3: how many machines in all are rented for one run. A
#                  Spot machine may be reclaimed at two minutes' notice
#                  (the second one rented here was, four minutes in);
#                  another is then rented and COMMAND run again on it
#                  with the results brought back so far, which the proof
#                  driver goes on from
#   EC2_IF_NONE    fail (the default) or ok: whether no machine to be had,
#                  for want of capacity or of quota, is this script's
#                  failure (status 2) or only said (status 0)
#
# What it costs is the machine's hours, to the minute it is terminated.
# What is left behind costs nothing: a network of its own (a VPC, its
# gateway to the internet, and a subnet in each zone it has tried), made
# on the first run and used by every one after, and a security group
# with no rule in it. All are tagged purpose=kani-proofs, and none of the
# account's own networks is used or changed.
#
# What it needs of the account: to describe, run and tag instances, to
# terminate the ones tagged purpose=kani-proofs, to make that network and
# a security group and open and close its port 22, and, for a first Spot
# request, to create the Spot service's role (the AmazonEC2FullAccess
# policy covers these). Nothing else: the machine is reached over SSH
# with a key made for the run, opened to this machine's address only, and
# reads the repository as anyone can (it is public).
#
# The exit status is COMMAND's; 3 if every machine rented went away
# under it, 4 if COMMAND was still running when the wait ran out.
# logs/kani is brought back every five minutes as well as at the end, so
# a machine that goes away takes at most five minutes of results.
set -euo pipefail

cd "$(dirname "$0")/.."

command=${1:?usage: scripts/ec2-run.sh COMMAND}
types=${EC2_TYPE:-r7a.8xlarge}
regions=${EC2_REGION:-us-east-1}
market=${EC2_MARKET:-spot}
max_price=${EC2_MAX_PRICE:-1.25}
disk=${EC2_DISK_GB:-500}
setup=${EC2_SETUP:-kani}
max_hours=${EC2_MAX_HOURS:-6}
wait_minutes=${EC2_WAIT_MINUTES:-320}
export AWS_DEFAULT_REGION=${regions%% *} AWS_PAGER=""
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
    trap '' INT TERM
    # What `patiently` left waiting.
    kill $(jobs -p) 2> /dev/null
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
# A run cancelled: GitHub signals the step's own process, waits ten
# seconds and kills what is left. The machine is given back in those ten
# seconds or it runs on until its own clock stops it, as the first
# cancelled run's did (run 46054250686: still there a minute after). So
# the signal ends this script, which gives the machine back as it ends;
# the workflow runs this script in place of the step's shell, so that the
# signal is this script's; and nothing here that takes long is waited for
# in the foreground, where bash would finish it before acting on a signal.
trap 'exit 130' INT TERM
patiently() { "$@" <&0 & wait $!; }

# This repository is public and so is a run's log: the account's number
# is kept out of what AWS says of a refusal, and no address is printed.
aws() {
    local status=0
    command aws "$@" 2> "$work/aws.err" || status=$?
    sed -E 's/[0-9]{12}/<account>/g' "$work/aws.err" >&2
    return $status
}

aws sts get-caller-identity --query UserId --output text > /dev/null
echo "the AWS credentials are accepted"

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

# What a launch needs of a region: the newest Ubuntu 24.04 that Canonical
# publishes there, with its root device, and a network of this script's
# own, made on the first run there and kept. An account need not have a
# default network (the first run here found none), and this uses none the
# account has. Each piece is looked for before it is made, so that a run
# stopped half-way through making them is finished by the next.
tags='Tags=[{Key=purpose,Value=kani-proofs},{Key=Name,Value=kani-proofs}]'
in_region() {
    local found gateway table
    found=$(aws ec2 describe-images --owners 099720109477 \
        --filters 'Name=name,Values=ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-*' 'Name=state,Values=available' \
        --query 'sort_by(Images,&CreationDate)[-1].[ImageId,RootDeviceName]' --output text)
    read -r image root <<< "$found" || true
    case "${image:-}" in ami-*) ;; *) echo "no Ubuntu 24.04 image found in $AWS_DEFAULT_REGION" >&2; exit 2 ;; esac

    vpc=$(aws ec2 describe-vpcs --filters Name=tag:purpose,Values=kani-proofs --query 'Vpcs[0].VpcId' --output text)
    if [ "$vpc" = None ]; then
        vpc=$(aws ec2 create-vpc --cidr-block 10.77.0.0/16 --tag-specifications "ResourceType=vpc,$tags" --query Vpc.VpcId --output text)
        aws ec2 wait vpc-available --vpc-ids "$vpc"
        echo "made the network kani-proofs in $AWS_DEFAULT_REGION"
    fi
    gateway=$(aws ec2 describe-internet-gateways --filters Name=tag:purpose,Values=kani-proofs \
        --query 'InternetGateways[0].InternetGatewayId' --output text)
    if [ "$gateway" = None ]; then
        gateway=$(aws ec2 create-internet-gateway --tag-specifications "ResourceType=internet-gateway,$tags" \
            --query InternetGateway.InternetGatewayId --output text)
    fi
    aws ec2 attach-internet-gateway --vpc-id "$vpc" --internet-gateway-id "$gateway" 2> "$work/gateway.err" \
        || grep -q Resource.AlreadyAssociated "$work/gateway.err" || { cat "$work/gateway.err" >&2; exit 2; }
    table=$(aws ec2 describe-route-tables --filters "Name=vpc-id,Values=$vpc" Name=association.main,Values=true \
        --query 'RouteTables[0].RouteTableId' --output text)
    aws ec2 create-route --route-table-id "$table" --destination-cidr-block 0.0.0.0/0 --gateway-id "$gateway" > /dev/null 2> "$work/route.err" \
        || grep -q RouteAlreadyExists "$work/route.err" || { cat "$work/route.err" >&2; exit 2; }

    # One security group, kept between runs with no rule in it.
    group=$(aws ec2 describe-security-groups --filters Name=group-name,Values=kani-proofs "Name=vpc-id,Values=$vpc" \
        --query 'SecurityGroups[0].GroupId' --output text)
    if [ "$group" = None ]; then
        group=$(aws ec2 create-security-group --group-name kani-proofs --vpc-id "$vpc" \
            --description "SSH to the Kani proof machines, opened for a run and closed after" --query GroupId --output text)
    fi
}

# The network's subnet in a zone, made if it is not there: 10.77.0.0/24 in
# the region's zone a, 10.77.1.0/24 in b, and so on.
subnet_in() {
    local zone=$1 found letter
    found=$(aws ec2 describe-subnets --filters "Name=vpc-id,Values=$vpc" "Name=availability-zone,Values=$zone" \
        --query 'Subnets[0].SubnetId' --output text)
    if [ "$found" = None ]; then
        letter=$(printf '%d' "'${zone: -1}")
        found=$(aws ec2 create-subnet --vpc-id "$vpc" --availability-zone "$zone" --cidr-block "10.77.$((letter - 97)).0/24" \
            --tag-specifications "ResourceType=subnet,$tags" --query Subnet.SubnetId --output text)
    fi
    echo "$found"
}

# Each region, each type, each zone that offers it, in turn, until one has
# a machine: a zone may have none of a type to spare, above all on the
# Spot market, where the third run here found none of the one type it
# asked for in any zone of its one region. Leaves the machine's id in
# `id`, or nothing there if none was to be had.
rent() {
    quota="" id=""
    for region in $regions; do
        export AWS_DEFAULT_REGION=$region
        in_region
        for type in $types; do
            zones=$(aws ec2 describe-instance-type-offerings --location-type availability-zone \
                --filters "Name=instance-type,Values=$type" --query 'InstanceTypeOfferings[].Location' --output text)
            [ -n "$zones" ] || { echo "$type is not offered in $region"; continue; }
            launch=(--image-id "$image" --instance-type "$type" --count 1
                    --instance-initiated-shutdown-behavior terminate --user-data "file://$work/user-data"
                    --block-device-mappings "DeviceName=$root,Ebs={VolumeSize=$disk,VolumeType=gp3,Iops=6000,Throughput=500,DeleteOnTermination=true}"
                    --tag-specifications "ResourceType=instance,Tags=[{Key=purpose,Value=kani-proofs},{Key=Name,Value=kani-proofs-${GITHUB_RUN_ID:-by-hand}}]"
                                         "ResourceType=volume,Tags=[{Key=purpose,Value=kani-proofs}]")
            if [ "$market" = spot ]; then
                launch+=(--instance-market-options "MarketType=spot,SpotOptions={MaxPrice=$max_price,SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}")
            fi
            for zone in $zones; do
                subnet=$(subnet_in "$zone")
                if id=$(aws ec2 run-instances "${launch[@]}" \
                        --network-interfaces "DeviceIndex=0,SubnetId=$subnet,AssociatePublicIpAddress=true,Groups=$group" \
                        --query 'Instances[0].InstanceId' --output text 2> "$work/launch.err"); then
                    return 0
                fi
                id=""
                why=$(sed -n 's/.*An error occurred (\([A-Za-z.]*\)).*/\1/p' "$work/launch.err" | head -n 1)
                echo "no $market $type in $zone: ${why:-$(tail -n 1 "$work/launch.err")}"
                case "$why" in
                    # What the account may have at once, which another zone
                    # does not change and a smaller type may.
                    MaxSpotInstanceCountExceeded | VcpuLimitExceeded) quota=yes; break ;;
                    # What no other zone, type or region changes.
                    UnauthorizedOperation | PendingVerification | OptInRequired | AuthFailure) cat "$work/launch.err" >&2; exit 2 ;;
                esac
            done
        done
    done
    return 0
}

there=(-i "$work/key" -o StrictHostKeyChecking=accept-new -o "UserKnownHostsFile=$work/known_hosts" -o BatchMode=yes
       -o ConnectTimeout=10 -o ServerAliveInterval=30 -o LogLevel=QUIET)
on_it() { ssh "${there[@]}" "ubuntu@$address" "$@"; }

bring_back() {
    mkdir -p logs/kani
    on_it 'cd repo && [ -d logs/kani ] && tar -cz --exclude="*.out*" logs/kani 2> /dev/null' > "$work/logs.tgz" 2> /dev/null \
        && [ -s "$work/logs.tgz" ] && tar -xzf "$work/logs.tgz"
}

# What is done with a machine once it is rented, as a process of its own,
# so that this script waits for it behind a `wait` and whatever fails in
# it is one status to look at. That status is COMMAND's; 100 if the
# machine stopped answering, 101 if COMMAND was still running when the
# wait ran out, 102 if the machine never answered.
use_it() (
    set -euo pipefail
    reached=""
    for _ in $(seq 60); do
        if on_it true 2> /dev/null; then reached=yes; break; fi
        sleep 5
    done
    [ -n "$reached" ] || { echo "$id did not answer on port 22 in ten minutes" >&2; exit 102; }
    on_it 'cloud-init status --wait > /dev/null 2>&1; echo "$(nproc) cores, $(free -g | awk "NR==2{print \$2}") GB, $(df -h --output=avail / | tail -1 | tr -d " ") of disk free"'

    # The checkout, by the commit: the repository is public, so the
    # machine holds no credential of any kind.
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

    # What a machine before this one had got to: the proof driver takes a
    # harness with a result as done.
    if ls logs/kani/shard-*.txt > /dev/null 2>&1; then
        tar -cz logs/kani | on_it 'cd repo && tar -xz'
        echo "going on from $(cat logs/kani/shard-*.txt | wc -l) results brought back from a machine before"
    fi

    # COMMAND, on its own there: this script only watches, so that a
    # connection lost is not a run lost. Its three streams are the
    # command's own and no shell's around it: begun as `cd repo && nohup
    # ... &`, the shell that ran that list held the connection's output
    # open for as long as COMMAND ran, and this script waited there
    # instead of watching (run 46053007731 brought nothing back until its
    # end, and run 46058836105 learned its machine was gone only when the
    # connection dropped).
    printf '%s\n' "$command" | on_it 'cat > run.sh'
    on_it 'nohup setsid bash -c "cd ~/repo; [ -f ~/.cargo/env ] && . ~/.cargo/env; bash ~/run.sh; echo \$? > ~/run.exit" > ~/run.log 2>&1 < /dev/null &'
    echo "running: $command"

    lost=0 minute=0
    while :; do
        sleep 60
        minute=$((minute + 1))
        if state=$(on_it 'cat run.exit 2> /dev/null || echo running' 2> /dev/null); then
            lost=0
        else
            lost=$((lost + 1))
            its=$(aws ec2 describe-instances --instance-ids "$id" --query 'Reservations[0].Instances[0].State.Name' --output text 2> /dev/null || echo unknown)
            if [ "$its" != running ] || [ $lost -ge 5 ]; then
                exit 100
            fi
            continue
        fi
        # Every five minutes, and at the end: the results so far, here,
        # where a machine reclaimed does not take them with it.
        if [ "$state" != running ] || [ $((minute % 5)) -eq 0 ]; then
            bring_back || true
            echo "minute $minute: $(cat logs/kani/shard-*.txt 2> /dev/null | awk '{n[$1]++} END {for (k in n) printf "%d %s  ", n[k], k}')"
        fi
        if [ "$state" != running ]; then
            on_it 'tail -n 60 run.log'
            echo "COMMAND ended with status $state after $minute minutes"
            exit "$state"
        fi
        if [ "$(date +%s)" -ge "$deadline" ]; then
            on_it 'tail -n 20 run.log' || true
            exit 101
        fi
    done
)

deadline=$(($(date +%s) + wait_minutes * 60))
attempts=${EC2_ATTEMPTS:-3}
attempt=1
while :; do
    rent
    if [ -z "$id" ]; then
        echo "no $market machine was to be had: none of $types, in $regions" >&2
        if [ -n "$quota" ]; then
            echo "the account's vCPU quota is below a machine asked for: Service Quotas, EC2," >&2
            echo "'All Standard (A, C, D, H, I, M, R, T, Z) Spot Instance Requests' for Spot," >&2
            echo "'Running On-Demand Standard (A, C, D, H, I, M, R, T, Z) instances' otherwise" >&2
        fi
        if [ "${EC2_IF_NONE:-fail}" = ok ] && [ $attempt -eq 1 ]; then
            echo "nothing was rented and nothing run"
            exit 0
        fi
        exit 2
    fi
    echo "launched $id: $market $type, $disk GB, in $zone; it turns itself off in $max_hours hours"

    # Port 22, opened to this machine's address for the run and closed after.
    me=${EC2_SSH_FROM:-$(curl -fsS https://checkip.amazonaws.com)}
    aws ec2 authorize-security-group-ingress --group-id "$group" --protocol tcp --port 22 --cidr "$me/32" > /dev/null 2> "$work/ingress.err" \
        || grep -q InvalidPermission.Duplicate "$work/ingress.err" || { cat "$work/ingress.err" >&2; exit 2; }

    patiently aws ec2 wait instance-running --instance-ids "$id" || true
    address=$(aws ec2 describe-instances --instance-ids "$id" --query 'Reservations[0].Instances[0].PublicIpAddress' --output text)
    status=0
    case "$address" in
        [0-9]*)
            use_it &
            wait $! || status=$?
            ;;
        *) status=100 ;;
    esac
    [ "$status" -ne 0 ] || exit 0
    if [ "$status" -eq 101 ]; then
        echo "COMMAND was still running after $wait_minutes minutes; its results so far are kept" >&2
        exit 4
    fi
    its=$(aws ec2 describe-instances --instance-ids "$id" --query 'Reservations[0].Instances[0].[State.Name,StateReason.Message]' \
        --output text 2> /dev/null | tr '\t' ' ' || true)
    # A machine still there and answering: the status is COMMAND's, or
    # the setup's.
    case "$status:$its" in 100:* | 102:*) ;; *:running*) exit "$status" ;; esac
    echo "the machine went away under the run: it is ${its:-not to be found}" >&2
    aws ec2 terminate-instances --instance-ids "$id" > /dev/null 2>&1 || true
    id=""
    if [ $attempt -ge "$attempts" ]; then
        echo "that was machine $attempt of the $attempts a run may rent" >&2
        exit 3
    fi
    attempt=$((attempt + 1))
    echo "renting another, machine $attempt of $attempts"
done
