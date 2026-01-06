#!/bin/sh

FILE_ABSOLUTE_PATH=$(find `pwd` -name $1)
FILE_ABSOLUTE_DIR=$(dirname $FILE_ABSOLUTE_PATH)
FILE_NAME=$(basename $1 | cut -f 1 -d '.')

echo ${FILE_NAME}

if [ "$(uname)" == 'Darwin' ]; then
    sips $1 -s format bmp --out ${FILE_ABSOLUTE_DIR}/${FILE_NAME}.bmp
else
    magick $1 ${FILE_NAME}.bmp
fi
