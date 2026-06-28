/*---------------------------------------------------------------------------+
|   EasyMesh - A Two-Dimensional Quality Mesh Generator                      |
|                                                                            |
|   Copyright (C) 2008 Bojan Niceno - bojan.niceno@psi.ch                    |
|                                                                            |
|   TecPlot ASCII output (.dat)                                              |
|   Format: FEPOINT with FETRIANGLE connectivity                             |
|                                                                            |
|   The file can be loaded directly into TecPlot 360 or TecPlot Focus.      |
|   Variable list: X, Y, BoundaryMarker                                      |
|   Each triangular element is written as a zone element entry.              |
+---------------------------------------------------------------------------*/
#include <cstdio>
#include <cstring>
#include "easymesh.h"

using namespace std;

/*==========================================================================*/
int EasyMesh::save_tec()
{
 int  e, n;
 int  r_Nn = 0, r_Ne = 0;

 /*------------------------------------------------------------------+
 |  Count active (non-OFF, renumbered) nodes and elements            |
 +------------------------------------------------------------------*/
 for(n=0; n<node.size(); n++)
   if(node[n].mark != OFF && node[n].new_numb != OFF)
     r_Nn++;

 for(e=0; e<elem.size(); e++)
   if(elem[e].mark != OFF && elem[e].new_numb != OFF)
     r_Ne++;

 /*------------------------------------------------------------------+
 |  Build temporary renumbered node table                            |
 +------------------------------------------------------------------*/
 Node r_node(r_Nn);
 r_node.increase_size(r_Nn);

 for(n=0; n<node.size(); n++)
   if(node[n].mark != OFF && node[n].new_numb != OFF)
    {
     r_node[node[n].new_numb].x    = node[n].x;
     r_node[node[n].new_numb].y    = node[n].y;
     r_node[node[n].new_numb].mark = node[n].mark;
    }

 /*------------------------------------------------------------------+
 |  Build temporary renumbered element table                         |
 +------------------------------------------------------------------*/
 Element r_elem(r_Ne);
 r_elem.increase_size(r_Ne);

 for(e=0; e<elem.size(); e++)
   if(elem[e].mark != OFF && elem[e].new_numb != OFF)
    {
     r_elem[elem[e].new_numb].i        = node[elem[e].i].new_numb;
     r_elem[elem[e].new_numb].j        = node[elem[e].j].new_numb;
     r_elem[elem[e].new_numb].k        = node[elem[e].k].new_numb;
     r_elem[elem[e].new_numb].material = elem[e].material;
    }

 /*------------------------------------------------------------------+
 |  Open output file: NAME.dat                                       |
 |  After save() returns, name[len-1] holds the last extension char  |
 |  ('s'). We reuse the same buffer pattern to write our extension.  |
 +------------------------------------------------------------------*/
 char tec_name[84];
 strncpy(tec_name, name, 80);
 tec_name[80] = '\0';
 /* name[len-1] was last set to 's' by save(). Replace extension.   */
 tec_name[len-2] = '\0';          /* strip ".s" -> base name        */
 strcat(tec_name, ".dat");

 FILE *out;
 if((out = fopen(tec_name, "w")) == NULL)
  {
   fprintf(stderr, "Cannot save TecPlot file %s !\n", tec_name);
   return 1;
  }

 /*------------------------------------------------------------------+
 |  TecPlot header                                                   |
 +------------------------------------------------------------------*/
 fprintf(out, "TITLE = \"EasyMesh triangulation\"\n");
 fprintf(out, "VARIABLES = \"X\", \"Y\", \"BoundaryMarker\"\n");
 fprintf(out, "ZONE T=\"Mesh\", N=%d, E=%d, DATAPACKING=POINT,"
              " ZONETYPE=FETRIANGLE\n", r_Nn, r_Ne);

 /*------------------------------------------------------------------+
 |  Node data: one line per node                                     |
 |  Format:  X   Y   BoundaryMarker                                  |
 +------------------------------------------------------------------*/
 for(n=0; n<r_Nn; n++)
   fprintf(out, " %18.15e  %18.15e  %d\n",
           r_node[n].x, r_node[n].y, r_node[n].mark);

 /*------------------------------------------------------------------+
 |  Connectivity: one line per triangle                              |
 |  TecPlot uses 1-based node indices                                |
 +------------------------------------------------------------------*/
 for(e=0; e<r_Ne; e++)
   fprintf(out, " %d  %d  %d\n",
           r_elem[e].i + 1,
           r_elem[e].j + 1,
           r_elem[e].k + 1);

 fflush(out);
 fclose(out);

 return 0;
}
