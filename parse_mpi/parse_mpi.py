#!/usr/bin/env python3

# usage mpirun path_to_bin | python parse_mpi.py

import os, sys, re

text_in = sys.stdin

ranks_log = {}
text_split = []
for line in text_in:
    
    # line += '\n'
    split = re.split("(\[\d+\] .*?)", line)
    
    if split[0] == '': split = split[1:]
    assert len(split) % 2 == 0
    
    for i in range(int(len(split) / 2)):
        text_split += [split[i * 2] + split[i * 2 + 1]]
            
for line in text_split:
    match = re.match("\[(\d+)\] (.*)", line)
    assert match
        
    buf_id = match.groups()[0]
    
    if buf_id not in ranks_log.keys():
        ranks_log[buf_id] = ""
    ranks_log[buf_id] += match.groups()[1]
    if line[-1] == '\n': ranks_log[buf_id] += '\n'

for rank in sorted(ranks_log):
    print("*** log for rank " + rank + ' ***\n')
    print(ranks_log[rank])
    print("*** end for rank " + rank + ' ***\n')
