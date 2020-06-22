/*
 * This file is part of FFmpeg.
 *
 * FFmpeg is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public
 * License as published by the Free Software Foundation; either
 * version 2.1 of the License, or (at your option) any later version.
 *
 * FFmpeg is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public
 * License along with FFmpeg; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA
 */

#ifndef AVUTIL_TIME_INTERNAL_H
#define AVUTIL_TIME_INTERNAL_H

#include <time.h>
#include <sys/time.h> // added by Chris
#include "config.h"

struct tm {

  int tm_sec;      /* 秒 [0-61] 最大2秒までのうるう秒を考慮 */

  int tm_min;      /* 分 [0-59] */

  int tm_hour;     /* 時 [0-23] */

  int tm_mday;     /* 日 [1-31] */

  int tm_mon;      /* 月 [0-11] 0から始まることに注意 */

  int tm_year;     /* 年 [1900からの経過年数] */

  int tm_wday;     /* 曜日 [0:日 1:月 ... 6:土] */

  int tm_yday;     /* 年内の通し日数 [0-365] 0から始まることに注意*/

  int tm_isdst;    /* 夏時間フラグ　[夏時間を採用しているときに正、採用していないときに 0、この情報が得られないときに負] */

};

#if !HAVE_GMTIME_R && !defined(gmtime_r)
static inline struct tm *gmtime_r(const time_t* clock, struct tm *result)
{
    struct tm *ptr = gmtime(clock);
    if (!ptr)
        return NULL;
    *result = *ptr;
    return result;
}
#endif

#if !HAVE_LOCALTIME_R && !defined(localtime_r)
static inline struct tm *localtime_r(const time_t* clock, struct tm *result)
{
    struct tm *ptr = localtime(clock);
    if (!ptr)
        return NULL;
    *result = *ptr;
    return result;
}
#endif

#endif /* AVUTIL_TIME_INTERNAL_H */
