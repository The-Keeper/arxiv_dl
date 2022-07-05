#!/bin/python
import os,sys,shutil
import urllib3,certifi
from urllib.parse import urlparse,urlunparse
#import uuid
import requests
import tarfile,gzip
import argparse

def makedirs(path: str):
    """creates directory if they don't exist"""
    if not os.path.exists(path):
        os.makedirs(path)

def main():
    parser = argparse.ArgumentParser(description='downloads and extracts archives from arXiv.org')
    parser.add_argument('ids',            type=str, nargs='+', help='archive id (such as 1910.02474)')   
    parser.add_argument('--dl-dir',         type=str, help="directory to download archives to", default="dl")    
    parser.add_argument('--extract-dir',    type=str, help="directory to extract archives to", default="extracted")    
    args = parser.parse_args()

    makedirs(args.dl_dir)
    makedirs(args.extract_dir)

    SITE_NAME = 'arxiv.org'
    s = requests.Session()

    for a in args.ids:
        path = os.path.join('e-print', a)
        out_path = os.path.join(args.dl_dir, a)
        link = urlunparse(['https', SITE_NAME, path, '','',''])
        print('Requesting ', link, '...')
        r = s.head(url=link, verify = True)
        if r.status_code != 200:
            print('Error: status code ', r.status_code)
        else:
            # assume no directories
            Downloaded = False
            f_size = int(r.headers['Content-Length'].strip())
            Downloaded = (os.path.exists(out_path) and os.path.getsize(out_path) == f_size)
            if Downloaded:
                print('Item', a, 'is already downloaded.')
            else:
                print('Downloading ', link, '...')
                r = s.get(url=link, verify = True)
                if r.status_code == 200:
                    with open(out_path, 'bw+') as f:
                        f.write(r.content)
                    Downloaded = True
            # extract
            if Downloaded:
                extr_path = os.path.join(args.extract_dir, a)
                makedirs(extr_path)
                try:
                    tf = tarfile.open(out_path)
                    tf.extractall(extr_path)
                except:
                    gz = gzip.open(out_path)
                    extr_path = os.path.join(extr_path, a)
                    with open(extr_path, 'bw+') as f:
                        f.write(gz.read())

if __name__ == '__main__':   # to run only as a script
    main()